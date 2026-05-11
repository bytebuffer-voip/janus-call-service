FROM debian:bullseye

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install -y \
    build-essential \
    git \
    curl \
    libssl-dev \
    libluajit-5.1-dev \
    libcurl4-openssl-dev \
    default-libmysqlclient-dev \
    libxml2-dev \
    libpcre3-dev \
    bison \
    flex \
    wget \
    libjson-c-dev \
    kamailio-json-modules \
    lua-socket \
    lua-sec \
    libjansson-dev \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

RUN ln -s /usr/include/luajit-2.1/* /usr/include/ && \
    ln -s /usr/lib/x86_64-linux-gnu/libluajit-5.1.so /usr/lib/x86_64-linux-gnu/liblua.so

WORKDIR /usr/src

RUN wget https://www.kamailio.org/pub/kamailio/5.8.7/src/kamailio-5.8.7_src.tar.gz \
    && tar xzf kamailio-5.8.7_src.tar.gz

RUN make cfg include_modules="app_lua json http_client jansson db_mysql" PREFIX=/usr/local
RUN make all LUA_NAME=luajit-5.1 && make install

RUN ln -sf /usr/local/sbin/kamailio /usr/sbin/kamailio \
    && ln -sf /usr/local/sbin/kamcmd /usr/sbin/kamcmd

RUN mkdir -p /usr/local/etc/kamailio/

EXPOSE 5060/udp 5060/tcp

CMD ["kamailio", "-DD", "-E"]