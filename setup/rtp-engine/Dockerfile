FROM debian:bookworm AS builder
ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update -y && apt-get install -y \
    build-essential \
    git \
    curl \
    wget \
    pkg-config \
    gperf \
    libcurl4-openssl-dev \
    libglib2.0-dev \
    libjson-glib-dev \
    zlib1g-dev \
    libssl-dev \
    libpcre3-dev \
    libevent-dev \
    libpcap-dev \
    libavcodec-dev \
    libavformat-dev \
    libavutil-dev \
    libavfilter-dev \
    libopus-dev \
    libspandsp-dev \
    libhiredis-dev \
    libxmlrpc-core-c3-dev \
    default-libmysqlclient-dev \
    libwebsockets-dev \
    libmnl-dev \
    libnftnl-dev \
    libiptc-dev \
    libxtables-dev \
    pandoc \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src

RUN git clone --branch mr13.3.1.4 --depth 1 https://github.com/sipwise/rtpengine.git rtpengine-mr13.3.1.4

WORKDIR /usr/src/rtpengine-mr13.3.1.4

RUN git submodule update --init --recursive

WORKDIR /usr/src/rtpengine-mr13.3.1.4/daemon

# Build binary
RUN make clean && make

RUN echo "===== LDD CHECK FOR RTPENGINE =====" && ldd rtpengine

FROM debian:bookworm
ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install -y \
    libglib2.0-0 \
    libjson-glib-1.0-0 \
    libpcap0.8 \
    libcurl4 \
    libevent-2.1-7 \
    libssl3 \
    libopus0 \
    libspandsp2 \
    libhiredis0.14 \
    libxmlrpc-core-c3 \
    default-mysql-client \
    libwebsockets17 \
    libevent-pthreads-2.1-7 \
    libnftnl11 \
    libip4tc2 \
    libip6tc2 \
    libavcodec59 \
    libavformat59 \
    && rm -rf /var/lib/apt/lists/* \

COPY --from=builder /usr/src/rtpengine-mr13.3.1.4/daemon/rtpengine /usr/sbin/rtpengine

RUN mkdir -p /var/run/rtpengine /tmp/rtpengine

EXPOSE 10000-20000/udp
EXPOSE 2223/udp

ENTRYPOINT ["/usr/sbin/rtpengine"]
CMD ["--interface=0.0.0.0", "--listen-ng=0.0.0.0:2223"]