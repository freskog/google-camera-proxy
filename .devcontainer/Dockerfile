FROM archlinux:base-devel-20230101.0.115167

RUN pacman-key --init
RUN pacman -Sy --noconfirm pipewire-jack pipewire-media-session gst-libav gstreamer \
                           gst-plugins-bad gst-plugins-base gst-plugins-good gst-plugins-ugly gst-rtsp-server \
                           git llvm clang pkg-config rustup

RUN useradd -ms /bin/bash vscode
RUN mkdir /workspace
RUN chown vscode:vscode /workspace

USER vscode
ADD client_secret.json /home/vscode/client_secret.json
WORKDIR /workspace

RUN rustup default stable
RUN rustup component add rustfmt
