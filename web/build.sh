#!/usr/bin/env bash

service_name="passer"

function usage {
  local base=$(basename "${0}")
  echo "HELP"
  echo "  Builds and prepares a ${service_name} web volume"
  echo
  echo "USAGE"
  echo "  ${base} [OPTIONS]"
  echo
  echo "OPTIONS"
  echo "  -d   Use docker instead of podman"
  echo "  -h   Print this help message"
  echo
  echo "EXAMPLE"
  echo "  ${base} -d"
}

for p in ${@}; do
  if [[ "${p}" == "-h" ]]; then
    usage
    exit
  fi
done

pod="podman"

if [ "${1}" ]; then
  if [[ "${1}" == "-d" ]]; then
    pod="docker"
  else
    error "Unknown parameter:" "${1}"
  fi
fi

${pod} build -t volume-updater .
${pod} run \
  --volume "${service_name}":/data \
  --rm \
  volume-updater \
  sh -c 'rm -rf /data/* && cp -r /web/build/* /data/.'
