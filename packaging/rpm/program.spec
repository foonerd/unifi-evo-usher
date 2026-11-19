# One program of unifi-evo-usher on its own, as an RPM: the file the
# Debian package of the same program has, in the same place.
# scripts/package.sh builds it with rpmbuild and says what is not written
# here:
#
#   usher_name      the program
#   usher_version   the version the workspace carries
#   usher_summary   one line on what the program is
#   usher_licence   the licences of what the package carries, as SPDX
#                   writes them
#   usher_tree      the tree to pack, laid out from the root of a file
#                   system
#
# Nothing is compiled, stripped or compressed again here: the program is
# the one that was built and proven, to the byte.

%global debug_package %{nil}
%global __os_install_post %{nil}
%global _build_id_links none

Name:        %{usher_name}
Version:     %{usher_version}
Release:     1
Summary:     %{usher_summary}
License:     %{usher_licence}
URL:         https://github.com/foonerd/unifi-evo-usher
Packager:    Just a nerd <157698061+foonerd@users.noreply.github.com>

# A static program: it asks nothing of the machine.
AutoReqProv: no

%description
One static program, installed as /opt/evo/bin/%{name}. It depends on no
other package.

%install
cp -a %{usher_tree}/. %{buildroot}/

%files
%defattr(-,root,root,-)
%dir /usr/share/licenses/%{name}
%license /usr/share/licenses/%{name}/*
%dir /opt/evo
%dir /opt/evo/bin
/opt/evo/bin/%{name}
