FROM qmcgaw/rustdevcontainer:debian

RUN apt-get update && export DEBIAN_FRONTEND=noninteractive \
    && apt-get -y install git libgstreamer1.0-dev libgstreamer-plugins-base1.0-dev \
      gstreamer1.0-plugins-base gstreamer1.0-plugins-good \
      gstreamer1.0-plugins-bad gstreamer1.0-plugins-ugly \
      gstreamer1.0-libav libgstrtspserver-1.0-dev libges-1.0-dev \
      libgstreamer-plugins-bad1.0-dev \
      libssl-dev

RUN useradd -ms /bin/bash vscode
USER vscode
ADD client_secret.json /home/vscode/client_secret.json
WORKDIR /workspace
