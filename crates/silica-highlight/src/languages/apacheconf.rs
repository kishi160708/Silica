//! Apache Configuration.
//!
//! Ported from Prism 1.29.0's `prism-apacheconf.js`. Written by
//! `tools/prism-port/port.js`, which walks the finished grammar objects;
//! change that script and re-run it rather than editing this file.

use crate::grammar::{e, p, pa, pai, pi, Grammar, LB};

pub static APACHECONF: Grammar = Grammar::new(&[
    e("comment", &[p(&[r"#.*"])]),
    e(
        "directive-inline",
        &[pa(
            &[
                r"(?im)(^[\t ]*)\b(?:AcceptFilter|AcceptPathInfo|AccessFileName|Action|Add(?:Alt|",
                r"AltByEncoding|AltByType|Charset|DefaultCharset|Description|Encoding|Handler|Icon|",
                r"IconByEncoding|IconByType|InputFilter|Language|ModuleInfo|OutputFilter|OutputFilterByType|",
                r"Type)|Alias|AliasMatch|Allow(?:CONNECT|EncodedSlashes|Methods|Override|OverrideList)?|",
                r"Anonymous(?:_LogEmail|_MustGiveEmail|_NoUserID|_VerifyEmail)?|AsyncRequestWorkerFactor|",
                r"Auth(?:BasicAuthoritative|BasicFake|BasicProvider|BasicUseDigestAlgorithm|DBDUserPWQuery|",
                r"DBDUserRealmQuery|DBMGroupFile|DBMType|DBMUserFile|Digest(?:Algorithm|Domain|NonceLifetime|",
                r"Provider|Qop|ShmemSize)|Form(?:Authoritative|Body|DisableNoStore|FakeBasicAuth|Location|",
                r"LoginRequiredLocation|LoginSuccessLocation|LogoutLocation|Method|Mimetype|Password|",
                r"Provider|SitePassphrase|Size|Username)|GroupFile|LDAP(?:AuthorizePrefix|BindAuthoritative|",
                r"BindDN|BindPassword|CharsetConfig|CompareAsUser|CompareDNOnServer|DereferenceAliases|",
                r"GroupAttribute|GroupAttributeIsDN|InitialBindAsUser|InitialBindPattern|MaxSubGroupDepth|",
                r"RemoteUserAttribute|RemoteUserIsDN|SearchAsUser|SubGroupAttribute|SubGroupClass|Url)|",
                r"Merging|Name|nCache(?:Context|Enable|ProvideFor|SOCache|Timeout)|nzFcgiCheckAuthnProvider|",
                r"nzFcgiDefineProvider|Type|UserFile|zDBDLoginToReferer|zDBDQuery|zDBDRedirectQuery|zDBMType|",
                r"zSendForbiddenOnFailure)|BalancerGrowth|BalancerInherit|BalancerMember|BalancerPersist|",
                r"BrowserMatch|BrowserMatchNoCase|BufferedLogs|BufferSize|Cache(?:DefaultExpire|DetailHeader|",
                r"DirLength|DirLevels|Disable|Enable|File|Header|IgnoreCacheControl|IgnoreHeaders|",
                r"IgnoreNoLastMod|IgnoreQueryString|IgnoreURLSessionIdentifiers|KeyBaseURL|",
                r"LastModifiedFactor|Lock|LockMaxAge|LockPath|MaxExpire|MaxFileSize|MinExpire|MinFileSize|",
                r"NegotiatedDocs|QuickHandler|ReadSize|ReadTime|Root|Socache(?:MaxSize|MaxTime|MinTime|",
                r"ReadSize|ReadTime)?|StaleOnError|StoreExpired|StoreNoStore|StorePrivate)|CGIDScriptTimeout|",
                r"CGIMapExtension|CharsetDefault|CharsetOptions|CharsetSourceEnc|CheckCaseOnly|CheckSpelling|",
                r"ChrootDir|ContentDigest|CookieDomain|CookieExpires|CookieName|CookieStyle|CookieTracking|",
                r"CoreDumpDirectory|CustomLog|Dav|DavDepthInfinity|DavGenericLockDB|DavLockDB|DavMinTimeout|",
                r"DBDExptime|DBDInitSQL|DBDKeep|DBDMax|DBDMin|DBDParams|DBDPersist|DBDPrepareSQL|DBDriver|",
                r"DefaultIcon|DefaultLanguage|DefaultRuntimeDir|DefaultType|Define|Deflate(?:BufferSize|",
                r"CompressionLevel|FilterNote|InflateLimitRequestBody|InflateRatio(?:Burst|Limit)|MemLevel|",
                r"WindowSize)|Deny|DirectoryCheckHandler|DirectoryIndex|DirectoryIndexRedirect|",
                r"DirectorySlash|DocumentRoot|DTracePrivileges|DumpIOInput|DumpIOOutput|EnableExceptionHook|",
                r"EnableMMAP|EnableSendfile|Error|ErrorDocument|ErrorLog|ErrorLogFormat|Example|",
                r"ExpiresActive|ExpiresByType|ExpiresDefault|ExtendedStatus|ExtFilterDefine|ExtFilterOptions|",
                r"FallbackResource|FileETag|FilterChain|FilterDeclare|FilterProtocol|FilterProvider|",
                r"FilterTrace|ForceLanguagePriority|ForceType|ForensicLog|GprofDir|GracefulShutdownTimeout|",
                r"Group|Header|HeaderName|Heartbeat(?:Address|Listen|MaxServers|Storage)|HostnameLookups|",
                r"IdentityCheck|IdentityCheckTimeout|ImapBase|ImapDefault|ImapMenu|Include|IncludeOptional|",
                r"Index(?:HeadInsert|Ignore|IgnoreReset|Options|OrderDefault|StyleSheet)|InputSed|",
                r"ISAPI(?:AppendLogToErrors|AppendLogToQuery|CacheFile|FakeAsync|LogNotSupported|",
                r"ReadAheadBuffer)|KeepAlive|KeepAliveTimeout|KeptBodySize|LanguagePriority|",
                r"LDAP(?:CacheEntries|CacheTTL|ConnectionPoolTTL|ConnectionTimeout|LibraryDebug|",
                r"OpCacheEntries|OpCacheTTL|ReferralHopLimit|Referrals|Retries|RetryDelay|SharedCacheFile|",
                r"SharedCacheSize|Timeout|TrustedClientCert|TrustedGlobalCert|TrustedMode|VerifyServerCert)|",
                r"Limit(?:InternalRecursion|Request(?:Body|Fields|FieldSize|Line)|XMLRequestBody)|Listen|",
                r"ListenBackLog|LoadFile|LoadModule|LogFormat|LogLevel|LogMessage|LuaAuthzProvider|",
                r"LuaCodeCache|Lua(?:Hook(?:AccessChecker|AuthChecker|CheckUserID|Fixups|InsertFilter|Log|",
                r"MapToStorage|TranslateName|TypeChecker)|Inherit|InputFilter|MapHandler|OutputFilter|",
                r"PackageCPath|PackagePath|QuickHandler|Root|Scope)|Max(?:ConnectionsPerChild|",
                r"KeepAliveRequests|MemFree|RangeOverlaps|RangeReversals|Ranges|RequestWorkers|SpareServers|",
                r"SpareThreads|Threads)|MergeTrailers|MetaDir|MetaFiles|MetaSuffix|MimeMagicFile|",
                r"MinSpareServers|MinSpareThreads|MMapFile|ModemStandard|ModMimeUsePathInfo|MultiviewsMatch|",
                r"Mutex|NameVirtualHost|NoProxy|NWSSLTrustedCerts|NWSSLUpgradeable|Options|Order|OutputSed|",
                r"PassEnv|PidFile|PrivilegesMode|Protocol|ProtocolEcho|Proxy(?:AddHeaders|BadHeader|Block|",
                r"Domain|ErrorOverride|ExpressDBMFile|ExpressDBMType|ExpressEnable|FtpDirCharset|",
                r"FtpEscapeWildcards|FtpListOnWildcard|HTML(?:BufSize|CharsetOut|DocType|Enable|Events|",
                r"Extended|Fixups|Interp|Links|Meta|StripComments|URLMap)|IOBufferSize|MaxForwards|",
                r"Pass(?:Inherit|InterpolateEnv|Match|Reverse|ReverseCookieDomain|ReverseCookiePath)?|",
                r"PreserveHost|ReceiveBufferSize|Remote|RemoteMatch|Requests|SCGIInternalRedirect|",
                r"SCGISendfile|Set|SourceAddress|Status|Timeout|Via)|ReadmeName|ReceiveBufferSize|Redirect|",
                r"RedirectMatch|RedirectPermanent|RedirectTemp|ReflectorHeader|RemoteIP(?:Header|",
                r"InternalProxy|InternalProxyList|ProxiesHeader|TrustedProxy|TrustedProxyList)|RemoveCharset|",
                r"RemoveEncoding|RemoveHandler|RemoveInputFilter|RemoveLanguage|RemoveOutputFilter|",
                r"RemoveType|RequestHeader|RequestReadTimeout|Require|Rewrite(?:Base|Cond|Engine|Map|Options|",
                r"Rule)|RLimitCPU|RLimitMEM|RLimitNPROC|Satisfy|ScoreBoardFile|Script(?:Alias|AliasMatch|",
                r"InterpreterSource|Log|LogBuffer|LogLength|Sock)?|SecureListen|SeeRequestTail|",
                r"SendBufferSize|Server(?:Admin|Alias|Limit|Name|Path|Root|Signature|Tokens)|",
                r"Session(?:Cookie(?:Name|Name2|Remove)|Crypto(?:Cipher|Driver|Passphrase|PassphraseFile)|",
                r"DBD(?:CookieName|CookieName2|CookieRemove|DeleteLabel|InsertLabel|PerUser|SelectLabel|",
                r"UpdateLabel)|Env|Exclude|Header|Include|MaxAge)?|SetEnv|SetEnvIf|SetEnvIfExpr|",
                r"SetEnvIfNoCase|SetHandler|SetInputFilter|SetOutputFilter|SSIEndTag|SSIErrorMsg|SSIETag|",
                r"SSILastModified|SSILegacyExprParser|SSIStartTag|SSITimeFormat|SSIUndefinedEcho|",
                r"SSL(?:CACertificateFile|CACertificatePath|CADNRequestFile|CADNRequestPath|",
                r"CARevocationCheck|CARevocationFile|CARevocationPath|CertificateChainFile|CertificateFile|",
                r"CertificateKeyFile|CipherSuite|Compression|CryptoDevice|Engine|FIPS|HonorCipherOrder|",
                r"InsecureRenegotiation|OCSP(?:DefaultResponder|Enable|OverrideResponder|ResponderTimeout|",
                r"ResponseMaxAge|ResponseTimeSkew|UseRequestNonce)|OpenSSLConfCmd|Options|PassPhraseDialog|",
                r"Protocol|Proxy(?:CACertificateFile|CACertificatePath|CARevocation(?:Check|File|Path)|",
                r"CheckPeer(?:CN|Expire|Name)|CipherSuite|Engine|MachineCertificate(?:ChainFile|File|Path)|",
                r"Protocol|Verify|VerifyDepth)|RandomSeed|RenegBufferSize|Require|RequireSSL|Session(?:Cache|",
                r"CacheTimeout|TicketKeyFile|Tickets)|SRPUnknownUserSeed|SRPVerifierFile|Stapling(?:Cache|",
                r"ErrorCacheTimeout|FakeTryLater|ForceURL|ResponderTimeout|ResponseMaxAge|ResponseTimeSkew|",
                r"ReturnResponderErrors|StandardCacheTimeout)|StrictSNIVHostCheck|UserName|UseStapling|",
                r"VerifyClient|VerifyDepth)|StartServers|StartThreads|Substitute|Suexec|SuexecUserGroup|",
                r"ThreadLimit|ThreadsPerChild|ThreadStackSize|TimeOut|TraceEnable|TransferLog|TypesConfig|",
                r"UnDefine|UndefMacro|UnsetEnv|Use|UseCanonicalName|UseCanonicalPhysicalPort|User|UserDir|",
                r"VHostCGIMode|VHostCGIPrivs|VHostGroup|VHostPrivs|VHostSecure|VHostUser|",
                r"Virtual(?:DocumentRoot|ScriptAlias)(?:IP)?|WatchdogInterval|XBitHack|xml2EncAlias|",
                r"xml2EncDefault|xml2StartParse)\b",
            ],
            LB,
            &["property"],
        )],
    ),
    e(
        "directive-block",
        &[pai(
            &[
                r"(?i)</?\b(?:Auth[nz]ProviderAlias|Directory|DirectoryMatch|Else|ElseIf|Files|FilesMatch|If|",
                r"IfDefine|IfModule|IfVersion|Limit|LimitExcept|Location|LocationMatch|Macro|Proxy|",
                r"Require(?:All|Any|None)|VirtualHost)\b.*>",
            ],
            0,
            &["tag"],
            &APACHECONF_DIRECTIVE_BLOCK,
        )],
    ),
    e(
        "directive-flags",
        &[pa(&[r"\[(?:[\w=],?)+\]"], 0, &["keyword"])],
    ),
    e("string", &[pi(&[r#"("|').*\1"#], 0, &APACHECONF_STRING)]),
    e("variable", &[p(&[r"[$%]\{?(?:\w\.?[-+:]?)+\}?"])]),
    e("regex", &[p(&[r"\^?.*\$|\^.*\$?"])]),
]);

pub(super) static APACHECONF_DIRECTIVE_BLOCK: Grammar = Grammar::new(&[
    e(
        "directive-block",
        &[pai(
            &[r"^</?\w+"],
            0,
            &["tag"],
            &APACHECONF_DIRECTIVE_BLOCK_DIRECTIVE_BLOCK,
        )],
    ),
    e(
        "directive-block-parameter",
        &[pai(
            &[r".*[^>]"],
            0,
            &["attr-value"],
            &APACHECONF_DIRECTIVE_BLOCK_DIRECTIVE_BLOCK_PARAMETER,
        )],
    ),
    e("punctuation", &[p(&[r">"])]),
]);

pub(super) static APACHECONF_DIRECTIVE_BLOCK_DIRECTIVE_BLOCK: Grammar =
    Grammar::new(&[e("punctuation", &[p(&[r"^</?"])])]);

pub(super) static APACHECONF_DIRECTIVE_BLOCK_DIRECTIVE_BLOCK_PARAMETER: Grammar = Grammar::new(&[
    e("punctuation", &[p(&[r":"])]),
    e(
        "string",
        &[pi(
            &[r#"("|').*\1"#],
            0,
            &APACHECONF_DIRECTIVE_BLOCK_DIRECTIVE_BLOCK_PARAMETER_STRING,
        )],
    ),
]);

pub(super) static APACHECONF_DIRECTIVE_BLOCK_DIRECTIVE_BLOCK_PARAMETER_STRING: Grammar =
    Grammar::new(&[e("variable", &[p(&[r"[$%]\{?(?:\w\.?[-+:]?)+\}?"])])]);

pub(super) static APACHECONF_STRING: Grammar =
    Grammar::new(&[e("variable", &[p(&[r"[$%]\{?(?:\w\.?[-+:]?)+\}?"])])]);
