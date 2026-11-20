# The browser version, served by nginx: the site of a release's
# openwgt-web.zip (the CI's docker job unpacks it into a folder of its
# own, the build's context). No game data: players bring their CD
# image or play without it.
#
#   docker run -p 8080:80 ghcr.io/gustavn64/openwgt   (or :0.12.0, a release)
#   (then http://localhost:8080/)
FROM nginx:1.31-alpine
COPY openwgt-web/ /usr/share/nginx/html/
EXPOSE 80
