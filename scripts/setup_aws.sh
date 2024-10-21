#!/bin/bash

# Set your AWS region
AWS_REGION="us-west-2"  # Change this to your preferred region

# Default to AWS, use --localstack flag to switch to LocalStack
USE_LOCALSTACK=false
LOCALSTACK_ENDPOINT="http://localhost:4566"

# Parse command line arguments
while [[ "$#" -gt 0 ]]; do
    case $1 in
        --localstack) USE_LOCALSTACK=true ;;
        *) echo "Unknown parameter: $1"; exit 1 ;;
    esac
    shift
done

# Set AWS CLI options based on environment
if [ "$USE_LOCALSTACK" = true ]; then
    AWS_ENDPOINT="--endpoint-url=$LOCALSTACK_ENDPOINT"
    echo "Using LocalStack at $LOCALSTACK_ENDPOINT"
else
    AWS_ENDPOINT=""
    echo "Using AWS in region $AWS_REGION"
fi

# Function to create a queue
create_queue() {
    local queue_name=$1
    local visibility_timeout=$2
    local max_receive_count=$3
    local dlq_arn=$4

    local attributes
    if [ -n "$dlq_arn" ]; then
        attributes=$(cat <<EOF
{
  "VisibilityTimeout": "$visibility_timeout",
  "RedrivePolicy": "{\"deadLetterTargetArn\":\"$dlq_arn\",\"maxReceiveCount\":$max_receive_count}"
}
EOF
)
    else
        attributes=$(cat <<EOF
{
  "VisibilityTimeout": "$visibility_timeout"
}
EOF
)
    fi

    aws $AWS_ENDPOINT sqs create-queue \
        --queue-name "$queue_name" \
        --attributes "$attributes" \
        --region $AWS_REGION
}

# Create DLQ first
create_queue "madara_orchestrator_job_handle_failure_queue" "30"

# Get DLQ ARN
DLQ_ARN=$(aws $AWS_ENDPOINT sqs get-queue-attributes \
    --queue-url $(aws $AWS_ENDPOINT sqs get-queue-url --queue-name madara_orchestrator_job_handle_failure_queue --region $AWS_REGION --output text --query 'QueueUrl') \
    --attribute-names QueueArn \
    --region $AWS_REGION \
    --output text \
    --query 'Attributes.QueueArn')

# Create other queues
create_queue "madara_orchestrator_snos_job_processing_queue" "1800" "3" "$DLQ_ARN"
create_queue "madara_orchestrator_snos_job_verification_queue" "30" "3" "$DLQ_ARN"
create_queue "madara_orchestrator_proving_job_processing_queue" "1800" "3" "$DLQ_ARN"
create_queue "madara_orchestrator_proving_job_verification_queue" "30" "3" "$DLQ_ARN"
create_queue "madara_orchestrator_data_submission_job_processing_queue" "1800" "3" "$DLQ_ARN"
create_queue "madara_orchestrator_data_submission_job_verification_queue" "30" "3" "$DLQ_ARN"
create_queue "madara_orchestrator_update_state_job_processing_queue" "1800" "3" "$DLQ_ARN"
create_queue "madara_orchestrator_update_state_job_verification_queue" "30" "3" "$DLQ_ARN"
create_queue "madara_orchestrator_worker_trigger_queue" "30"

# Create S3 bucket
aws $AWS_ENDPOINT s3api create-bucket \
    --bucket madara-orchestrator-test-bucket \
    --region $AWS_REGION \
    $([ "$USE_LOCALSTACK" = false ] && echo "--create-bucket-configuration LocationConstraint=$AWS_REGION")

# Create SNS topic
aws $AWS_ENDPOINT sns create-topic \
    --name madara-orchestrator \
    --region $AWS_REGION

# Function to create EventBridge rule
create_event_rule() {
    local rule_name=$1
    local worker_type=$2

    # For LocalStack, we need to use cloudwatch events instead of eventbridge
    local event_service="events"
    if [ "$USE_LOCALSTACK" = false ]; then
        event_service="events"
    fi

    aws $AWS_ENDPOINT $event_service put-rule \
        --name "$rule_name" \
        --schedule-expression "rate(1 minute)" \
        --state ENABLED \
        --region $AWS_REGION

    local queue_url=$(aws $AWS_ENDPOINT sqs get-queue-url --queue-name madara_orchestrator_worker_trigger_queue --region $AWS_REGION --output json | jq -r .QueueUrl)

    aws $AWS_ENDPOINT $event_service put-targets \
        --rule "$rule_name" \
        --targets "[{\"Id\":\"1\",\"Arn\":\"$queue_url\",\"Input\":\"{\\\"worker\\\":\\\"$worker_type\\\"}\"}]" \
        --region $AWS_REGION
}

create_event_rule "MadaraOrchestratorSnosWorkerTrigger" "Snos"
create_event_rule "MadaraOrchestratorProvingWorkerTrigger" "Proving"
create_event_rule "MadaraOrchestratorDataSubmissionWorkerTrigger" "DataSubmission"
create_event_rule "MadaraOrchestratorUpdateStateWorkerTrigger" "UpdateState"

echo "Setup completed successfully!"