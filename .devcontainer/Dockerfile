FROM mcr.microsoft.com/devcontainers/base:debian12

RUN sed -i 's/deb.debian.org/mirrors.ustc.edu.cn/g' /etc/apt/sources.list.d/debian.sources \
   && apt-get update && export DEBIAN_FRONTEND=noninteractive \
   && apt-get -y install clang lld pkg-config libssl-dev \
   && apt-get autoremove -y && apt-get clean -y

USER vscode
RUN curl https://sh.rustup.rs -sSf | sudo su - vscode -c "RUSTUP_UPDATE_ROOT=https://mirrors.ustc.edu.cn/rust-static/rustup RUSTUP_DIST_SERVER=https://mirrors.ustc.edu.cn/rust-static bash -s -- -y -c rust-analyzer"
ENV PATH="/home/vscode/.cargo/bin:${PATH}"