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

### Calling the service
The payload to send to the cluster-service lambda should look like this:
```
{
    "taskId": 76,
    "data": {
        "bucket": { S3-BUCKET },
        "key": "input/{ INPUT-FILE }.csv"
    },
    "numPerm": 64,
    "numBands": 16,
    "threshold": 0.49
}
```

### Receiving callbacks
On sucessfull completion you will receive a response like this:
```
{
    "body": {
        "taskId": 76,
        "data": {
            "bucket": "{ S3-BUCKET }",
            "key": "output/{ INPUT-FILE }.csv"
        }
    },
    "statusCode": 200
}
```

If the cluster-service lambda fails:
```
{
    "body": {
        "config": {
            "taskId": 76,
            "data": {
                "bucket": { S3-BUCKET },
                "key": "input/{ INPUT-FILE }.csv"
            },
            "numBands": 16,
            "numPerm": 64,
            "threshold": 0.49
        },
        "message": "2025-03-05T18:35:55.615Z 06fe0c8c-7786-46d8-8159-13d846887b24 Task timed out after 900.29 seconds"
    },  
    "statusCode": 504
}
```