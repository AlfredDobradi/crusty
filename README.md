# Crusty the balancing guy

I'm learning Rust and no better way to do that than trying to make a load balancer (You say an activity pub implementation? Don't worry I got that covered too: https://github.com/alfreddobradi/crabiverse)

### disclaimer

Currently doesn't balance anything.

### Usage

You can launch with ./crusty-lb -b 0.0.0.0 -p 8080 --targets localhost:8888,localhost:9999

As of now it will create a HashMap of the targets and runs a connectivity check on them every second.

The actual proxying code pushes data to a hardcoded target (WIP :shrug:) and returns once it's done. Not great but I'm making progress.