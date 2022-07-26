# Restrict

This is a small wrapper program to restrict a commands resource usage.

It wraps an arbitrary subcommand in a new shell with stdin, stdout and stderr
inherited and adds the child pid to a cgroup with the specified resource
restrictions.

```shell
$ cargo build --release
$ sudo ./target/release/restrict -d -m 100M -c 10 -- 'echo test1 && sleep 1 && echo test2'
run command /bin/bash -c 'echo test1 && sleep 1 && echo test2'
        cpu    restricted to  10 shares
        memory restricted to  100.0 MB
test1
test2
command exited with status exit status: 0
```
