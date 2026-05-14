#!/usr/bin/env sh
set -eu

apt_mirror="${1:-http://mirrors.aliyun.com}"

if [ -f /etc/apt/sources.list.d/debian.sources ]; then
  sed -i \
    -e "s|http://deb.debian.org/debian|${apt_mirror}/debian|g" \
    -e "s|https://deb.debian.org/debian|${apt_mirror}/debian|g" \
    -e "s|http://security.debian.org/debian-security|${apt_mirror}/debian-security|g" \
    -e "s|https://security.debian.org/debian-security|${apt_mirror}/debian-security|g" \
    -e "s|http://deb.debian.org/debian-security|${apt_mirror}/debian-security|g" \
    -e "s|https://deb.debian.org/debian-security|${apt_mirror}/debian-security|g" \
    /etc/apt/sources.list.d/debian.sources
fi

if [ -f /etc/apt/sources.list ]; then
  sed -i \
    -e "s|http://deb.debian.org/debian|${apt_mirror}/debian|g" \
    -e "s|https://deb.debian.org/debian|${apt_mirror}/debian|g" \
    -e "s|http://security.debian.org/debian-security|${apt_mirror}/debian-security|g" \
    -e "s|https://security.debian.org/debian-security|${apt_mirror}/debian-security|g" \
    -e "s|http://deb.debian.org/debian-security|${apt_mirror}/debian-security|g" \
    -e "s|https://deb.debian.org/debian-security|${apt_mirror}/debian-security|g" \
    /etc/apt/sources.list
fi
