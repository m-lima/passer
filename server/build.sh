#!/usr/bin/env bash

service_name="passer"

function usage {
  local base=$(basename "${0}")
  echo "HELP"
  echo "  Builds and prepares a ${service_name} container"
  echo
  echo "USAGE"
  echo "  ${base} [OPTIONS]"
  echo "  ${base} unit [-d]"
  echo
  echo "COMMANDS"
  echo "  unit Print a systemd unit file and quit"
  echo
  echo "OPTIONS"
  echo "  -d   Use docker instead of podman"
  echo "  -c   Stop and start the service"
  echo "  -h   Print this help message"
  echo
  echo "EXAMPLE"
  echo "  ${base} -c"
  echo "  ${base} unit -d > ${service_name}.service"
}

function error {
  echo -n "[31m${1}[m" >&2
  if [ "${2}" ]; then
    echo " ${2}" >&2
  else
    echo >&2
  fi

  echo >&2
  usage >&2
  exit 1
}

function unit {
  local pod bin
  pod="podman"

  if [ "${1}" ]; then
    if [[ "${1}" == "-d" ]]; then
      pod="docker"
    else
      error "Unknown parameter:" "${1}"
    fi
  fi

  if ! command -v ${pod} > /dev/null; then
    error "Not found:" "${pod}"
  fi

  if [[ "${pod}" == "podman" ]]; then
    ${pod} generate systemd --name "${service_name}" \
      | sed 's/^Description=.*$/Description=Passer/g' \
      | sed 's/^WantedBy=.*$/WantedBy=nginx-frontend.service/g'
  else
    bin=$(which "${pod}")

    cat <<EOF
[Unit]
Description=Passer
After=${pod}.service
Requires=${pod}.service

[Service]
Restart=always
ExecStart=${bin} start -a ${service_name}
ExecStop=${bin} stop ${service_name}

[Install]
WantedBy=nginx-frontend.service
EOF
  fi

  exit
}

function build {
  local base=$(dirname "${0}")
  base=$(realpath "${base}")
  local pod="podman"
  local cycle=""
  local output

  while [ "${1}" ]; do
    case "${1}" in
      "-d") pod="docker" ;;
      "-c") cycle="1" ;;
      *) error "Unknown parameter:" "${1}" ;;
    esac
    shift
  done

  if [ "${autovolumes}" ]; then
    echo "[33mNot all volumes detected as required were added:[m"
    echo "${autovolumes}"
  fi

  if [ ${cycle} ]; then
    echo "[34mStopping the service[m"
    systemctl stop "${service_name}"
  fi

  echo "[34mBuilding the image[m"
  if ! ${pod} build -t "${service_name}" "${base}"; then
    exit 1
  fi

  echo "[34mChecking for running instances[m"
  output=$(${pod} ps --format '{{.ID}} {{.Names}}' | grep "${service_name}")
  if [ "${output}" ]; then
    ${pod} stop $(cut -d' ' -f1 <<<"${output}")
  fi

  echo "[34mChecking for existing containers[m"
  output=$(${pod} ps -a --format '{{.ID}} {{.Names}}' | grep "${service_name}")
  if [ "${output}" ]; then
    ${pod} rm $(cut -d' ' -f1 <<<"${output}")
  fi

  echo "[34mCreating the container[m"
  ${pod} create \
    --network nginx \
    --volume "${service_name}-data":/data \
    --name "${service_name}" \
    passer \
    -s /data


  if [ ${cycle} ]; then
    echo "[34mStarting the service[m"
    systemctl start "${service_name}"
  fi
}

for p in ${@}; do
  if [[ "${p}" == "-h" ]]; then
    usage
    exit
  fi
done

if [[ "${1}" == "unit" ]]; then
  shift
  unit ${@}
else
  build ${@}
fi
