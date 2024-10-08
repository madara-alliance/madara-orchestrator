FROM rust:1.75-bullseye

# Set the working directory in the container
WORKDIR /usr/src/app

# Install system dependencies
RUN apt update && apt install -y  \
    libgmp3-dev \
    software-properties-common \
    wget \
    bash

# Install Python 3.9
RUN wget https://www.python.org/ftp/python/3.9.16/Python-3.9.16.tgz \
    && tar xzf Python-3.9.16.tgz \
    && cd Python-3.9.16 \
    && ./configure --enable-optimizations \
    && make altinstall \
    && cd .. \
    && rm -rf Python-3.9.16 Python-3.9.16.tgz

# Install pip
RUN wget https://bootstrap.pypa.io/get-pip.py \
    && python3.9 get-pip.py \
    && rm get-pip.py

# Set up Python environment and install Cairo
RUN python3.9 -m venv /usr/local/cairo_venv
RUN pip3.9 install ecdsa fastecdsa sympy
RUN pip3.9 install cairo-lang

RUN python3.9 --version && pip3.9 --version

# Copy the current directory contents into the container
COPY . .

# #############################################################
# TODO : remove this step after snos build is sorted
# Build cairo lang
RUN cargo fetch
RUN bash -c "cd /usr/local/cargo/git/checkouts \
    && cd snos-* \
    && cd * \
    && source /usr/local/cairo_venv/bin/activate \
    && ./scripts/setup-tests.sh"
# #############################################################

WORKDIR /usr/src/app

# Build the project
RUN cargo build --release

CMD ["cargo", "run", "--release"]