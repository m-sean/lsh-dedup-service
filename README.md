# lsh-dedup-service

### Build services
To generate the images for lambda services run the following command in the main directory
```
./build-services { AWS-ACCOUNT } { AWS-REGION }
```

### Push to AWS ECR
If necessary, login with:
```
aws ecr get-login-password \
    --region { REGION } \
    --profile { AWS-PROFILE } \
    | docker login --username AWS \
        --password-stdin { AWS-ACCOUNT }.dkr.ecr.{ AWS-REGION }.amazonaws.com
```

Push the created image(s):
```
docker push { AWS-ACCOUNT }.dkr.ecr.{ AWS-REGION }.amazonaws.com/lsh-dedup/{ SERVICE }
```
(Note: `{ SERVICE }` = `callback-service` or `cluster-service`)

