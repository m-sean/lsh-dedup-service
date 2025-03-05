FROM amazonlinux:2 
WORKDIR /usr/app

ARG SERVICE_NAME

COPY target/release/${SERVICE_NAME} .

# persist service name in env for cmd 
ENV EX_CMD="./${SERVICE_NAME}"

CMD $EX_CMD