FROM --platform=$BUILDPLATFORM golang:1.19.9-alpine3.16 as builder

ARG VERSION=v0.0.0

RUN apk add --no-cache make gcc musl-dev linux-headers git jq bash

# build op-proposer with the shared go.mod & go.sum files
COPY ./optimism/op-proposer /app/op-proposer
COPY ./optimism/op-bindings /app/op-bindings
COPY ./optimism/op-node /app/op-node
COPY ./optimism/op-service /app/op-service
COPY ./optimism/op-signer /app/op-signer
COPY ./optimism/go.mod /app/go.mod
COPY ./optimism/go.sum /app/go.sum
COPY ./.git /app/.git

WORKDIR /app/op-proposer

RUN go mod download

ARG TARGETOS TARGETARCH

RUN make op-proposer VERSION="$VERSION" GOOS=$TARGETOS GOARCH=$TARGETARCH

FROM alpine:3.16

COPY --from=builder /app/op-proposer/bin/op-proposer /usr/local/bin

CMD ["op-proposer"]
