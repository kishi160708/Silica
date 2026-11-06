//! Bash (`sh`, `shell`).
//!
//! Ported from Prism 1.29.0's `prism-bash.js`. Written by
//! `tools/prism-port/port.js`, which walks the finished grammar objects;
//! change that script and re-run it rather than editing this file.

use crate::grammar::{e, p, pa, pai, pf, pi, Grammar, GR, LB};

pub static BASH: Grammar = Grammar::new(&[
    e("shebang", &[pa(&[r"^#!\s*/.*"], 0, &["important"])]),
    e("comment", &[pf(&[r#"(^|[^"{\\$])#.*"#], LB)]),
    e(
        "function-name",
        &[
            pa(
                &[r"(\bfunction\s+)[\w-]+(?=(?:\s*\(?:\s*\))?\s*\{)"],
                LB,
                &["function"],
            ),
            pa(&[r"\b[\w-]+(?=\s*\(\s*\)\s*\{)"], 0, &["function"]),
        ],
    ),
    e(
        "for-or-select",
        &[pa(
            &[r"(\b(?:for|select)\s+)\w+(?=\s+in\s)"],
            LB,
            &["variable"],
        )],
    ),
    e(
        "assign-left",
        &[pai(
            &[r"(^|[\s;|\&]|[<>]\()\w+(?:\.\w+)*(?=\+?=)"],
            LB,
            &["variable"],
            &BASH_ASSIGN_LEFT,
        )],
    ),
    e(
        "parameter",
        &[pa(
            &[r"(^|\s)-{1,2}(?:\w+:[+-]?)?\w+(?:\.\w+)*(?=[=\s]|$)"],
            LB,
            &["variable"],
        )],
    ),
    e(
        "string",
        &[
            pi(
                &[r"((?:^|[^<])<<-?\s*)(\w+)\s[\s\S]*?(?:\r?\n|\r)\2"],
                LB | GR,
                &BASH_STRING_1,
            ),
            pi(
                &[r#"((?:^|[^<])<<-?\s*)(["'])(\w+)\2\s[\s\S]*?(?:\r?\n|\r)\3"#],
                LB | GR,
                &BASH_STRING_1_VARIABLE_2_STRING_2,
            ),
            pi(
                &[r#"(^|[^\\](?:\\\\)*)"(?:\\[\s\S]|\$\([^)]+\)|\$(?!\()|`[^`]+`|[^"\\`$])*""#],
                LB | GR,
                &BASH_STRING_1,
            ),
            pf(&[r"(^|[^$\\])'[^']*'"], LB | GR),
            pi(
                &[r"\$'(?:[^'\\]|\\[\s\S])*'"],
                GR,
                &BASH_STRING_1_VARIABLE_2_STRING_5,
            ),
        ],
    ),
    e(
        "environment",
        &[pa(
            &[
                r"\$?\b(?:BASH|BASHOPTS|BASH_ALIASES|BASH_ARGC|BASH_ARGV|BASH_CMDS|",
                r"BASH_COMPLETION_COMPAT_DIR|BASH_LINENO|BASH_REMATCH|BASH_SOURCE|BASH_VERSINFO|BASH_VERSION|",
                r"COLORTERM|COLUMNS|COMP_WORDBREAKS|DBUS_SESSION_BUS_ADDRESS|DEFAULTS_PATH|DESKTOP_SESSION|",
                r"DIRSTACK|DISPLAY|EUID|GDMSESSION|GDM_LANG|GNOME_KEYRING_CONTROL|GNOME_KEYRING_PID|",
                r"GPG_AGENT_INFO|GROUPS|HISTCONTROL|HISTFILE|HISTFILESIZE|HISTSIZE|HOME|HOSTNAME|HOSTTYPE|",
                r"IFS|INSTANCE|JOB|LANG|LANGUAGE|LC_ADDRESS|LC_ALL|LC_IDENTIFICATION|LC_MEASUREMENT|",
                r"LC_MONETARY|LC_NAME|LC_NUMERIC|LC_PAPER|LC_TELEPHONE|LC_TIME|LESSCLOSE|LESSOPEN|LINES|",
                r"LOGNAME|LS_COLORS|MACHTYPE|MAILCHECK|MANDATORY_PATH|NO_AT_BRIDGE|OLDPWD|OPTERR|OPTIND|",
                r"ORBIT_SOCKETDIR|OSTYPE|PAPERSIZE|PATH|PIPESTATUS|PPID|PS1|PS2|PS3|PS4|PWD|RANDOM|REPLY|",
                r"SECONDS|SELINUX_INIT|SESSION|SESSIONTYPE|SESSION_MANAGER|SHELL|SHELLOPTS|SHLVL|",
                r"SSH_AUTH_SOCK|TERM|UID|UPSTART_EVENTS|UPSTART_INSTANCE|UPSTART_JOB|UPSTART_SESSION|USER|",
                r"WINDOWID|XAUTHORITY|XDG_CONFIG_DIRS|XDG_CURRENT_DESKTOP|XDG_DATA_DIRS|XDG_GREETER_DATA_DIR|",
                r"XDG_MENU_PREFIX|XDG_RUNTIME_DIR|XDG_SEAT|XDG_SEAT_PATH|XDG_SESSION_DESKTOP|XDG_SESSION_ID|",
                r"XDG_SESSION_PATH|XDG_SESSION_TYPE|XDG_VTNR|XMODIFIERS)\b",
            ],
            0,
            &["constant"],
        )],
    ),
    e(
        "variable",
        &[
            pi(&[r"\$?\(\([\s\S]+?\)\)"], GR, &BASH_STRING_1_VARIABLE_1),
            pi(
                &[r"\$\((?:\([^)]+\)|[^()])+\)|`[^`]+`"],
                GR,
                &BASH_STRING_1_VARIABLE_2,
            ),
            pi(&[r"\$\{[^}]+\}"], GR, &BASH_STRING_1_VARIABLE_3),
            p(&[r"\$(?:\w+|[#?*!@$])"]),
        ],
    ),
    e(
        "function",
        &[pf(
            &[
                r"(^|[\s;|\&]|[<>]\()(?:add|apropos|apt|apt-cache|apt-get|aptitude|aspell|automysqlbackup|",
                r"awk|basename|bash|bc|bconsole|bg|bzip2|cal|cargo|cat|cfdisk|chgrp|chkconfig|chmod|chown|",
                r"chroot|cksum|clear|cmp|column|comm|composer|cp|cron|crontab|csplit|curl|cut|date|dc|dd|",
                r"ddrescue|debootstrap|df|diff|diff3|dig|dir|dircolors|dirname|dirs|dmesg|docker|",
                r"docker-compose|du|egrep|eject|env|ethtool|expand|expect|expr|fdformat|fdisk|fg|fgrep|file|",
                r"find|fmt|fold|format|free|fsck|ftp|fuser|gawk|git|gparted|grep|groupadd|groupdel|groupmod|",
                r"groups|grub-mkconfig|gzip|halt|head|hg|history|host|hostname|htop|iconv|id|ifconfig|ifdown|",
                r"ifup|import|install|ip|java|jobs|join|kill|killall|less|link|ln|locate|logname|logrotate|",
                r"look|lpc|lpr|lprint|lprintd|lprintq|lprm|ls|lsof|lynx|make|man|mc|mdadm|mkconfig|mkdir|",
                r"mke2fs|mkfifo|mkfs|mkisofs|mknod|mkswap|mmv|more|most|mount|mtools|mtr|mutt|mv|nano|nc|",
                r"netstat|nice|nl|node|nohup|notify-send|npm|nslookup|op|open|parted|passwd|paste|pathchk|",
                r"ping|pkill|pnpm|podman|podman-compose|popd|pr|printcap|printenv|ps|pushd|pv|quota|",
                r"quotacheck|quotactl|ram|rar|rcp|reboot|remsync|rename|renice|rev|rm|rmdir|rpm|rsync|scp|",
                r"screen|sdiff|sed|sendmail|seq|service|sftp|sh|shellcheck|shuf|shutdown|sleep|slocate|sort|",
                r"split|ssh|stat|strace|su|sudo|sum|suspend|swapon|sync|sysctl|tac|tail|tar|tee|time|timeout|",
                r"top|touch|tr|traceroute|tsort|tty|umount|uname|unexpand|uniq|units|unrar|unshar|unzip|",
                r"update-grub|uptime|useradd|userdel|usermod|users|uudecode|uuencode|v|vcpkg|vdir|vi|vim|",
                r"virsh|vmstat|wait|watch|wc|wget|whereis|which|who|whoami|write|xargs|xdg-open|yarn|yes|",
                r"zenity|zip|zsh|zypper)(?=$|[)\s;|\&])",
            ],
            LB,
        )],
    ),
    e(
        "keyword",
        &[pf(
            &[
                r"(^|[\s;|\&]|[<>]\()(?:case|do|done|elif|else|esac|fi|for|function|if|in|select|then|until|",
                r"while)(?=$|[)\s;|\&])",
            ],
            LB,
        )],
    ),
    e(
        "builtin",
        &[pa(
            &[
                r"(^|[\s;|\&]|[<>]\()(?:\.|:|alias|bind|break|builtin|caller|cd|command|continue|declare|",
                r"echo|enable|eval|exec|exit|export|getopts|hash|help|let|local|logout|mapfile|printf|pwd|",
                r"read|readarray|readonly|return|set|shift|shopt|source|test|times|trap|type|typeset|ulimit|",
                r"umask|unalias|unset)(?=$|[)\s;|\&])",
            ],
            LB,
            &["class-name"],
        )],
    ),
    e(
        "boolean",
        &[pf(
            &[r"(^|[\s;|\&]|[<>]\()(?:false|true)(?=$|[)\s;|\&])"],
            LB,
        )],
    ),
    e("file-descriptor", &[pa(&[r"\B&\d\b"], 0, &["important"])]),
    e(
        "operator",
        &[pi(
            &[r"\d?<>|>\||\+=|=[=\~]?|!=?|<<[<-]?|[\&\d]?>>|\d[<>]&?|[<>][\&=]?|&[>\&]?|\|[\&|]?"],
            0,
            &BASH_STRING_1_VARIABLE_2_OPERATOR,
        )],
    ),
    e("punctuation", &[p(&[r"\$?\(\(?|\)\)?|\.\.|[{}\[\];\\]"])]),
    e(
        "number",
        &[pf(&[r"(^|\s)(?:[1-9]\d*|0)(?:[.,]\d+)?\b"], LB)],
    ),
]);

pub(super) static BASH_ASSIGN_LEFT: Grammar = Grammar::new(&[e(
    "environment",
    &[pa(
        &[
            r"(^|[\s;|\&]|[<>]\()\b(?:BASH|BASHOPTS|BASH_ALIASES|BASH_ARGC|BASH_ARGV|BASH_CMDS|",
            r"BASH_COMPLETION_COMPAT_DIR|BASH_LINENO|BASH_REMATCH|BASH_SOURCE|BASH_VERSINFO|BASH_VERSION|",
            r"COLORTERM|COLUMNS|COMP_WORDBREAKS|DBUS_SESSION_BUS_ADDRESS|DEFAULTS_PATH|DESKTOP_SESSION|",
            r"DIRSTACK|DISPLAY|EUID|GDMSESSION|GDM_LANG|GNOME_KEYRING_CONTROL|GNOME_KEYRING_PID|",
            r"GPG_AGENT_INFO|GROUPS|HISTCONTROL|HISTFILE|HISTFILESIZE|HISTSIZE|HOME|HOSTNAME|HOSTTYPE|",
            r"IFS|INSTANCE|JOB|LANG|LANGUAGE|LC_ADDRESS|LC_ALL|LC_IDENTIFICATION|LC_MEASUREMENT|",
            r"LC_MONETARY|LC_NAME|LC_NUMERIC|LC_PAPER|LC_TELEPHONE|LC_TIME|LESSCLOSE|LESSOPEN|LINES|",
            r"LOGNAME|LS_COLORS|MACHTYPE|MAILCHECK|MANDATORY_PATH|NO_AT_BRIDGE|OLDPWD|OPTERR|OPTIND|",
            r"ORBIT_SOCKETDIR|OSTYPE|PAPERSIZE|PATH|PIPESTATUS|PPID|PS1|PS2|PS3|PS4|PWD|RANDOM|REPLY|",
            r"SECONDS|SELINUX_INIT|SESSION|SESSIONTYPE|SESSION_MANAGER|SHELL|SHELLOPTS|SHLVL|",
            r"SSH_AUTH_SOCK|TERM|UID|UPSTART_EVENTS|UPSTART_INSTANCE|UPSTART_JOB|UPSTART_SESSION|USER|",
            r"WINDOWID|XAUTHORITY|XDG_CONFIG_DIRS|XDG_CURRENT_DESKTOP|XDG_DATA_DIRS|XDG_GREETER_DATA_DIR|",
            r"XDG_MENU_PREFIX|XDG_RUNTIME_DIR|XDG_SEAT|XDG_SEAT_PATH|XDG_SESSION_DESKTOP|XDG_SESSION_ID|",
            r"XDG_SESSION_PATH|XDG_SESSION_TYPE|XDG_VTNR|XMODIFIERS)\b",
        ],
        LB,
        &["constant"],
    )],
)]);

pub(super) static BASH_STRING_1: Grammar = Grammar::new(&[
    e(
        "bash",
        &[pai(
            &[r#"(^(["']?)\w+\2)[ \t]+\S.*"#],
            LB,
            &["punctuation"],
            &BASH,
        )],
    ),
    e(
        "environment",
        &[pa(
            &[
                r"\$\b(?:BASH|BASHOPTS|BASH_ALIASES|BASH_ARGC|BASH_ARGV|BASH_CMDS|BASH_COMPLETION_COMPAT_DIR|",
                r"BASH_LINENO|BASH_REMATCH|BASH_SOURCE|BASH_VERSINFO|BASH_VERSION|COLORTERM|COLUMNS|",
                r"COMP_WORDBREAKS|DBUS_SESSION_BUS_ADDRESS|DEFAULTS_PATH|DESKTOP_SESSION|DIRSTACK|DISPLAY|",
                r"EUID|GDMSESSION|GDM_LANG|GNOME_KEYRING_CONTROL|GNOME_KEYRING_PID|GPG_AGENT_INFO|GROUPS|",
                r"HISTCONTROL|HISTFILE|HISTFILESIZE|HISTSIZE|HOME|HOSTNAME|HOSTTYPE|IFS|INSTANCE|JOB|LANG|",
                r"LANGUAGE|LC_ADDRESS|LC_ALL|LC_IDENTIFICATION|LC_MEASUREMENT|LC_MONETARY|LC_NAME|LC_NUMERIC|",
                r"LC_PAPER|LC_TELEPHONE|LC_TIME|LESSCLOSE|LESSOPEN|LINES|LOGNAME|LS_COLORS|MACHTYPE|",
                r"MAILCHECK|MANDATORY_PATH|NO_AT_BRIDGE|OLDPWD|OPTERR|OPTIND|ORBIT_SOCKETDIR|OSTYPE|",
                r"PAPERSIZE|PATH|PIPESTATUS|PPID|PS1|PS2|PS3|PS4|PWD|RANDOM|REPLY|SECONDS|SELINUX_INIT|",
                r"SESSION|SESSIONTYPE|SESSION_MANAGER|SHELL|SHELLOPTS|SHLVL|SSH_AUTH_SOCK|TERM|UID|",
                r"UPSTART_EVENTS|UPSTART_INSTANCE|UPSTART_JOB|UPSTART_SESSION|USER|WINDOWID|XAUTHORITY|",
                r"XDG_CONFIG_DIRS|XDG_CURRENT_DESKTOP|XDG_DATA_DIRS|XDG_GREETER_DATA_DIR|XDG_MENU_PREFIX|",
                r"XDG_RUNTIME_DIR|XDG_SEAT|XDG_SEAT_PATH|XDG_SESSION_DESKTOP|XDG_SESSION_ID|XDG_SESSION_PATH|",
                r"XDG_SESSION_TYPE|XDG_VTNR|XMODIFIERS)\b",
            ],
            0,
            &["constant"],
        )],
    ),
    e(
        "variable",
        &[
            pi(&[r"\$?\(\([\s\S]+?\)\)"], GR, &BASH_STRING_1_VARIABLE_1),
            pi(
                &[r"\$\((?:\([^)]+\)|[^()])+\)|`[^`]+`"],
                GR,
                &BASH_STRING_1_VARIABLE_2,
            ),
            pi(&[r"\$\{[^}]+\}"], GR, &BASH_STRING_1_VARIABLE_3),
            p(&[r"\$(?:\w+|[#?*!@$])"]),
        ],
    ),
    e(
        "entity",
        &[p(&[
            r#"\\(?:[abceEfnrtv\\"]|O?[0-7]{1,3}|U[0-9a-fA-F]{8}|u[0-9a-fA-F]{4}|x[0-9a-fA-F]{1,2})"#,
        ])],
    ),
]);

pub(super) static BASH_STRING_1_VARIABLE_1: Grammar = Grammar::new(&[
    e(
        "variable",
        &[pf(&[r"(^\$\(\([\s\S]+)\)\)"], LB), p(&[r"^\$\(\("])],
    ),
    e(
        "number",
        &[p(&[
            r"\b0x[\dA-Fa-f]+\b|(?:\b\d+(?:\.\d*)?|\B\.\d+)(?:[Ee]-?\d+)?",
        ])],
    ),
    e(
        "operator",
        &[p(&[
            r"--|\+\+|\*\*=?|<<=?|>>=?|&&|\|\||[=!+\-*/%<>^\&|]=?|[?\~:]",
        ])],
    ),
    e("punctuation", &[p(&[r"\(\(?|\)\)?|,|;"])]),
]);

pub(super) static BASH_STRING_1_VARIABLE_2: Grammar = Grammar::new(&[
    e("variable", &[p(&[r"^\$\(|^`|\)$|`$"])]),
    e("comment", &[pf(&[r#"(^|[^"{\\$])#.*"#], LB)]),
    e(
        "function-name",
        &[
            pa(
                &[r"(\bfunction\s+)[\w-]+(?=(?:\s*\(?:\s*\))?\s*\{)"],
                LB,
                &["function"],
            ),
            pa(&[r"\b[\w-]+(?=\s*\(\s*\)\s*\{)"], 0, &["function"]),
        ],
    ),
    e(
        "for-or-select",
        &[pa(
            &[r"(\b(?:for|select)\s+)\w+(?=\s+in\s)"],
            LB,
            &["variable"],
        )],
    ),
    e(
        "assign-left",
        &[pai(
            &[r"(^|[\s;|\&]|[<>]\()\w+(?:\.\w+)*(?=\+?=)"],
            LB,
            &["variable"],
            &BASH_ASSIGN_LEFT,
        )],
    ),
    e(
        "parameter",
        &[pa(
            &[r"(^|\s)-{1,2}(?:\w+:[+-]?)?\w+(?:\.\w+)*(?=[=\s]|$)"],
            LB,
            &["variable"],
        )],
    ),
    e(
        "string",
        &[
            pi(
                &[r"((?:^|[^<])<<-?\s*)(\w+)\s[\s\S]*?(?:\r?\n|\r)\2"],
                LB | GR,
                &BASH_STRING_1,
            ),
            pi(
                &[r#"((?:^|[^<])<<-?\s*)(["'])(\w+)\2\s[\s\S]*?(?:\r?\n|\r)\3"#],
                LB | GR,
                &BASH_STRING_1_VARIABLE_2_STRING_2,
            ),
            pi(
                &[r#"(^|[^\\](?:\\\\)*)"(?:\\[\s\S]|\$\([^)]+\)|\$(?!\()|`[^`]+`|[^"\\`$])*""#],
                LB | GR,
                &BASH_STRING_1,
            ),
            pf(&[r"(^|[^$\\])'[^']*'"], LB | GR),
            pi(
                &[r"\$'(?:[^'\\]|\\[\s\S])*'"],
                GR,
                &BASH_STRING_1_VARIABLE_2_STRING_5,
            ),
        ],
    ),
    e(
        "environment",
        &[pa(
            &[
                r"\$?\b(?:BASH|BASHOPTS|BASH_ALIASES|BASH_ARGC|BASH_ARGV|BASH_CMDS|",
                r"BASH_COMPLETION_COMPAT_DIR|BASH_LINENO|BASH_REMATCH|BASH_SOURCE|BASH_VERSINFO|BASH_VERSION|",
                r"COLORTERM|COLUMNS|COMP_WORDBREAKS|DBUS_SESSION_BUS_ADDRESS|DEFAULTS_PATH|DESKTOP_SESSION|",
                r"DIRSTACK|DISPLAY|EUID|GDMSESSION|GDM_LANG|GNOME_KEYRING_CONTROL|GNOME_KEYRING_PID|",
                r"GPG_AGENT_INFO|GROUPS|HISTCONTROL|HISTFILE|HISTFILESIZE|HISTSIZE|HOME|HOSTNAME|HOSTTYPE|",
                r"IFS|INSTANCE|JOB|LANG|LANGUAGE|LC_ADDRESS|LC_ALL|LC_IDENTIFICATION|LC_MEASUREMENT|",
                r"LC_MONETARY|LC_NAME|LC_NUMERIC|LC_PAPER|LC_TELEPHONE|LC_TIME|LESSCLOSE|LESSOPEN|LINES|",
                r"LOGNAME|LS_COLORS|MACHTYPE|MAILCHECK|MANDATORY_PATH|NO_AT_BRIDGE|OLDPWD|OPTERR|OPTIND|",
                r"ORBIT_SOCKETDIR|OSTYPE|PAPERSIZE|PATH|PIPESTATUS|PPID|PS1|PS2|PS3|PS4|PWD|RANDOM|REPLY|",
                r"SECONDS|SELINUX_INIT|SESSION|SESSIONTYPE|SESSION_MANAGER|SHELL|SHELLOPTS|SHLVL|",
                r"SSH_AUTH_SOCK|TERM|UID|UPSTART_EVENTS|UPSTART_INSTANCE|UPSTART_JOB|UPSTART_SESSION|USER|",
                r"WINDOWID|XAUTHORITY|XDG_CONFIG_DIRS|XDG_CURRENT_DESKTOP|XDG_DATA_DIRS|XDG_GREETER_DATA_DIR|",
                r"XDG_MENU_PREFIX|XDG_RUNTIME_DIR|XDG_SEAT|XDG_SEAT_PATH|XDG_SESSION_DESKTOP|XDG_SESSION_ID|",
                r"XDG_SESSION_PATH|XDG_SESSION_TYPE|XDG_VTNR|XMODIFIERS)\b",
            ],
            0,
            &["constant"],
        )],
    ),
    e(
        "function",
        &[pf(
            &[
                r"(^|[\s;|\&]|[<>]\()(?:add|apropos|apt|apt-cache|apt-get|aptitude|aspell|automysqlbackup|",
                r"awk|basename|bash|bc|bconsole|bg|bzip2|cal|cargo|cat|cfdisk|chgrp|chkconfig|chmod|chown|",
                r"chroot|cksum|clear|cmp|column|comm|composer|cp|cron|crontab|csplit|curl|cut|date|dc|dd|",
                r"ddrescue|debootstrap|df|diff|diff3|dig|dir|dircolors|dirname|dirs|dmesg|docker|",
                r"docker-compose|du|egrep|eject|env|ethtool|expand|expect|expr|fdformat|fdisk|fg|fgrep|file|",
                r"find|fmt|fold|format|free|fsck|ftp|fuser|gawk|git|gparted|grep|groupadd|groupdel|groupmod|",
                r"groups|grub-mkconfig|gzip|halt|head|hg|history|host|hostname|htop|iconv|id|ifconfig|ifdown|",
                r"ifup|import|install|ip|java|jobs|join|kill|killall|less|link|ln|locate|logname|logrotate|",
                r"look|lpc|lpr|lprint|lprintd|lprintq|lprm|ls|lsof|lynx|make|man|mc|mdadm|mkconfig|mkdir|",
                r"mke2fs|mkfifo|mkfs|mkisofs|mknod|mkswap|mmv|more|most|mount|mtools|mtr|mutt|mv|nano|nc|",
                r"netstat|nice|nl|node|nohup|notify-send|npm|nslookup|op|open|parted|passwd|paste|pathchk|",
                r"ping|pkill|pnpm|podman|podman-compose|popd|pr|printcap|printenv|ps|pushd|pv|quota|",
                r"quotacheck|quotactl|ram|rar|rcp|reboot|remsync|rename|renice|rev|rm|rmdir|rpm|rsync|scp|",
                r"screen|sdiff|sed|sendmail|seq|service|sftp|sh|shellcheck|shuf|shutdown|sleep|slocate|sort|",
                r"split|ssh|stat|strace|su|sudo|sum|suspend|swapon|sync|sysctl|tac|tail|tar|tee|time|timeout|",
                r"top|touch|tr|traceroute|tsort|tty|umount|uname|unexpand|uniq|units|unrar|unshar|unzip|",
                r"update-grub|uptime|useradd|userdel|usermod|users|uudecode|uuencode|v|vcpkg|vdir|vi|vim|",
                r"virsh|vmstat|wait|watch|wc|wget|whereis|which|who|whoami|write|xargs|xdg-open|yarn|yes|",
                r"zenity|zip|zsh|zypper)(?=$|[)\s;|\&])",
            ],
            LB,
        )],
    ),
    e(
        "keyword",
        &[pf(
            &[
                r"(^|[\s;|\&]|[<>]\()(?:case|do|done|elif|else|esac|fi|for|function|if|in|select|then|until|",
                r"while)(?=$|[)\s;|\&])",
            ],
            LB,
        )],
    ),
    e(
        "builtin",
        &[pa(
            &[
                r"(^|[\s;|\&]|[<>]\()(?:\.|:|alias|bind|break|builtin|caller|cd|command|continue|declare|",
                r"echo|enable|eval|exec|exit|export|getopts|hash|help|let|local|logout|mapfile|printf|pwd|",
                r"read|readarray|readonly|return|set|shift|shopt|source|test|times|trap|type|typeset|ulimit|",
                r"umask|unalias|unset)(?=$|[)\s;|\&])",
            ],
            LB,
            &["class-name"],
        )],
    ),
    e(
        "boolean",
        &[pf(
            &[r"(^|[\s;|\&]|[<>]\()(?:false|true)(?=$|[)\s;|\&])"],
            LB,
        )],
    ),
    e("file-descriptor", &[pa(&[r"\B&\d\b"], 0, &["important"])]),
    e(
        "operator",
        &[pi(
            &[r"\d?<>|>\||\+=|=[=\~]?|!=?|<<[<-]?|[\&\d]?>>|\d[<>]&?|[<>][\&=]?|&[>\&]?|\|[\&|]?"],
            0,
            &BASH_STRING_1_VARIABLE_2_OPERATOR,
        )],
    ),
    e("punctuation", &[p(&[r"\$?\(\(?|\)\)?|\.\.|[{}\[\];\\]"])]),
    e(
        "number",
        &[pf(&[r"(^|\s)(?:[1-9]\d*|0)(?:[.,]\d+)?\b"], LB)],
    ),
]);

pub(super) static BASH_STRING_1_VARIABLE_2_STRING_2: Grammar = Grammar::new(&[e(
    "bash",
    &[pai(
        &[r#"(^(["']?)\w+\2)[ \t]+\S.*"#],
        LB,
        &["punctuation"],
        &BASH,
    )],
)]);

pub(super) static BASH_STRING_1_VARIABLE_2_STRING_5: Grammar = Grammar::new(&[e(
    "entity",
    &[p(&[
        r#"\\(?:[abceEfnrtv\\"]|O?[0-7]{1,3}|U[0-9a-fA-F]{8}|u[0-9a-fA-F]{4}|x[0-9a-fA-F]{1,2})"#,
    ])],
)]);

pub(super) static BASH_STRING_1_VARIABLE_2_OPERATOR: Grammar =
    Grammar::new(&[e("file-descriptor", &[pa(&[r"^\d"], 0, &["important"])])]);

pub(super) static BASH_STRING_1_VARIABLE_3: Grammar = Grammar::new(&[
    e("operator", &[p(&[r":[-=?+]?|[!/]|##?|%%?|\^\^?|,,?"])]),
    e("punctuation", &[p(&[r"[\[\]]"])]),
    e(
        "environment",
        &[pa(
            &[
                r"(\{)\b(?:BASH|BASHOPTS|BASH_ALIASES|BASH_ARGC|BASH_ARGV|BASH_CMDS|",
                r"BASH_COMPLETION_COMPAT_DIR|BASH_LINENO|BASH_REMATCH|BASH_SOURCE|BASH_VERSINFO|BASH_VERSION|",
                r"COLORTERM|COLUMNS|COMP_WORDBREAKS|DBUS_SESSION_BUS_ADDRESS|DEFAULTS_PATH|DESKTOP_SESSION|",
                r"DIRSTACK|DISPLAY|EUID|GDMSESSION|GDM_LANG|GNOME_KEYRING_CONTROL|GNOME_KEYRING_PID|",
                r"GPG_AGENT_INFO|GROUPS|HISTCONTROL|HISTFILE|HISTFILESIZE|HISTSIZE|HOME|HOSTNAME|HOSTTYPE|",
                r"IFS|INSTANCE|JOB|LANG|LANGUAGE|LC_ADDRESS|LC_ALL|LC_IDENTIFICATION|LC_MEASUREMENT|",
                r"LC_MONETARY|LC_NAME|LC_NUMERIC|LC_PAPER|LC_TELEPHONE|LC_TIME|LESSCLOSE|LESSOPEN|LINES|",
                r"LOGNAME|LS_COLORS|MACHTYPE|MAILCHECK|MANDATORY_PATH|NO_AT_BRIDGE|OLDPWD|OPTERR|OPTIND|",
                r"ORBIT_SOCKETDIR|OSTYPE|PAPERSIZE|PATH|PIPESTATUS|PPID|PS1|PS2|PS3|PS4|PWD|RANDOM|REPLY|",
                r"SECONDS|SELINUX_INIT|SESSION|SESSIONTYPE|SESSION_MANAGER|SHELL|SHELLOPTS|SHLVL|",
                r"SSH_AUTH_SOCK|TERM|UID|UPSTART_EVENTS|UPSTART_INSTANCE|UPSTART_JOB|UPSTART_SESSION|USER|",
                r"WINDOWID|XAUTHORITY|XDG_CONFIG_DIRS|XDG_CURRENT_DESKTOP|XDG_DATA_DIRS|XDG_GREETER_DATA_DIR|",
                r"XDG_MENU_PREFIX|XDG_RUNTIME_DIR|XDG_SEAT|XDG_SEAT_PATH|XDG_SESSION_DESKTOP|XDG_SESSION_ID|",
                r"XDG_SESSION_PATH|XDG_SESSION_TYPE|XDG_VTNR|XMODIFIERS)\b",
            ],
            LB,
            &["constant"],
        )],
    ),
]);
