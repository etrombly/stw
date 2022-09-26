SyncThing Wrapper
---

This is a wrapper program to assist using syncthing for remote development. It creates a local and remote syncthing config, and uploads syncthing to the remote target.

### Example config

For now the config file needs to be named `config.yml` and be located in the same directory you run stw from. In the future this will be configurable.

```
remote_address: "192.168.0.99"
remote_user: eric
ssh_key: /home/eric/.ssh/id_rsa
folders:
  - local_path: /tmp/test
    remote_path: /tmp/test
```

ssh_key is optional, if you want to use username and password. If ssh-agent is running it will attempt to connect with the agent first, if that doesn't work it will fall back to prompting for the key password.

You need to download and run syncthing manually on the local machine for now. STW lists the command to run after initializing, i.e.:
```
Run `syncthing serve --home="/home/eric/.config/stw/15506ed50944d59e1b43b4f40fe31c29"` on local machine to sync
```