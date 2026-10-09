# api's image in the reference application's shape: the layers of the Docker stage, which
# sbt-native-packager's `api/Docker/stage` and `teq stage api` both write, copied as they are.
FROM eclipse-temurin:21-jdk
USER root
WORKDIR /opt/docker
COPY ./api/target/docker/stage/2/opt/docker /opt/docker
COPY ./api/target/docker/stage/4/opt/docker /opt/docker
EXPOSE 8080
ENTRYPOINT ["/opt/docker/bin/api"]
