# The steward as a unit, as an RPM: the files the Debian package has, in
# the same places, from the same tree. scripts/package.sh builds it with
# rpmbuild and says what is not written here:
#
#   usher_version   the version the workspace carries
#   usher_summary   one line on what the program is
#   usher_tree      the tree to pack, laid out from the root of a file
#                   system
#   usher_config    a list of what the tree has under /etc/evo, each file
#                   marked as configuration a newer package leaves alone
#
# Nothing is compiled, stripped or compressed again here: the program is
# the one that was built and proven, to the byte.

%global debug_package %{nil}
%global __os_install_post %{nil}
%global _build_id_links none

Name:             evo-usher-steward
Version:          %{usher_version}
Release:          1
Summary:          %{usher_summary}
License:          MIT AND BUSL-1.1
URL:              https://github.com/foonerd/unifi-evo-usher
Packager:         Just a nerd <157698061+foonerd@users.noreply.github.com>

# A static program: it asks nothing of the machine but the system
# manager, which runs it and makes its account.
AutoReqProv:      no
Requires:         systemd
Requires(post):   systemd
Requires(preun):  systemd
Requires(postun): systemd

%description
The evo steward with the usher's plugins, as one static program under
/opt/evo, with the usher's face, its catalogue, its configuration
(/etc/evo/evo.toml) and the service that runs it (evo.service) as an
account of its own. The face is served on port 8443.

%install
cp -a %{usher_tree}/. %{buildroot}/

%post
# After the files are in place: the account the steward runs as, and the
# service known to the machine, set to start with it, and started. Only a
# first installation starts it; a newer package finds it as it was left.
set -e
systemd-sysusers evo-usher-steward.conf
if [ -d /run/systemd/system ]; then
  systemctl daemon-reload
  if [ "$1" -eq 1 ]; then
    systemctl enable --now evo.service
  fi
elif [ "$1" -eq 1 ]; then
  # Installed into an image that is being built: the steward is started
  # when the image is.
  systemctl enable evo.service > /dev/null 2>&1 || :
fi

%preun
# Before the files go: where the package is removed and not replaced by a
# newer one, the steward is stopped and no longer started with the
# machine.
if [ "$1" -eq 0 ] && [ -d /run/systemd/system ]; then
  systemctl disable --now evo.service || :
fi

%postun
# After the files are gone. Where a newer package took their place, the
# steward that runs is the old program, and is started again as the new
# one. What the steward kept, /var/lib/evo, is the operator's and stays,
# as does configuration that was changed, which rpm keeps beside its old
# place as .rpmsave: an RPM is removed, never purged. The account stays
# too, as the owner of what was kept.
if [ -d /run/systemd/system ]; then
  systemctl daemon-reload || :
  if [ "$1" -ge 1 ]; then
    systemctl try-restart evo.service || :
  fi
fi

%files -f %{usher_config}
%defattr(-,root,root,-)
%dir /usr/share/licenses/%{name}
%license /usr/share/licenses/%{name}/LICENSE
%license /usr/share/licenses/%{name}/LICENSE-evo.txt
%dir /opt/evo
%dir /opt/evo/bin
/opt/evo/bin/evo-usher-steward
/opt/evo/catalogue
/opt/evo/ui
/usr/lib/systemd/system/evo.service
/usr/lib/sysusers.d/evo-usher-steward.conf
