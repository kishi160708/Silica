//! Arduino (`ino`).
//!
//! Ported from Prism 1.29.0's `prism-clike.js`, `prism-c.js`, `prism-cpp.js`, `prism-arduino.js`
//! and `prism-opencl.js`,
//! as the grammar stands once all of them have loaded. Written by
//! `tools/prism-port/port.js`, which walks the finished grammar objects;
//! change that script and re-run it rather than editing this file.

use crate::grammar::{e, p, pa, pai, pf, pi, Grammar, GR, LB};

pub static ARDUINO: Grammar = Grammar::new(&[
    e(
        "comment",
        &[pf(
            &[r"//(?:[^\r\n\\]|\\(?:\r\n?|\n|(?![\r\n])))*|/\*[\s\S]*?(?:\*/|$)"],
            GR,
        )],
    ),
    e(
        "char",
        &[pf(&[r"'(?:\\(?:\r\n|[\s\S])|[^'\\\r\n]){0,32}'"], GR)],
    ),
    e(
        "macro",
        &[pai(
            &[
                r"(?im)(^[\t ]*)#\s*[a-z](?:[^\r\n\\/]|/(?!\*)|/\*(?:[^*]|\*(?!/))*\*/|\\(?:\r\n|[\s\S]))*",
            ],
            LB | GR,
            &["property"],
            &ARDUINO_MACRO,
        )],
    ),
    e(
        "module",
        &[pi(
            &[
                r#"(\b(?:import|module)\s+)(?:"(?:\\(?:\r\n|[\s\S])|[^"\\\r\n])*"|<[^<>\r\n]*>|"#,
                r"\b(?!\b(?:alignas|alignof|asm|auto|bool|break|case|catch|char|char16_t|char32_t|char8_t|",
                r"class|co_await|co_return|co_yield|compl|concept|const|const_cast|consteval|constexpr|",
                r"constinit|continue|decltype|default|delete|do|double|dynamic_cast|else|enum|explicit|",
                r"export|extern|final|float|for|friend|goto|if|import|inline|int|int16_t|int32_t|int64_t|",
                r"int8_t|long|module|mutable|namespace|new|noexcept|nullptr|operator|override|private|",
                r"protected|public|register|reinterpret_cast|requires|return|short|signed|sizeof|static|",
                r"static_assert|static_cast|struct|switch|template|this|thread_local|throw|try|typedef|",
                r"typeid|typename|uint16_t|uint32_t|uint64_t|uint8_t|union|unsigned|using|virtual|void|",
                r"volatile|wchar_t|while)\b)\w+(?:\s*\.\s*\w+)*\b(?:\s*:\s*\b(?!\b(?:alignas|alignof|asm|",
                r"auto|bool|break|case|catch|char|char16_t|char32_t|char8_t|class|co_await|co_return|",
                r"co_yield|compl|concept|const|const_cast|consteval|constexpr|constinit|continue|decltype|",
                r"default|delete|do|double|dynamic_cast|else|enum|explicit|export|extern|final|float|for|",
                r"friend|goto|if|import|inline|int|int16_t|int32_t|int64_t|int8_t|long|module|mutable|",
                r"namespace|new|noexcept|nullptr|operator|override|private|protected|public|register|",
                r"reinterpret_cast|requires|return|short|signed|sizeof|static|static_assert|static_cast|",
                r"struct|switch|template|this|thread_local|throw|try|typedef|typeid|typename|uint16_t|",
                r"uint32_t|uint64_t|uint8_t|union|unsigned|using|virtual|void|volatile|wchar_t|",
                r"while)\b)\w+(?:\s*\.\s*\w+)*\b)?|:\s*\b(?!\b(?:alignas|alignof|asm|auto|bool|break|case|",
                r"catch|char|char16_t|char32_t|char8_t|class|co_await|co_return|co_yield|compl|concept|const|",
                r"const_cast|consteval|constexpr|constinit|continue|decltype|default|delete|do|double|",
                r"dynamic_cast|else|enum|explicit|export|extern|final|float|for|friend|goto|if|import|inline|",
                r"int|int16_t|int32_t|int64_t|int8_t|long|module|mutable|namespace|new|noexcept|nullptr|",
                r"operator|override|private|protected|public|register|reinterpret_cast|requires|return|short|",
                r"signed|sizeof|static|static_assert|static_cast|struct|switch|template|this|thread_local|",
                r"throw|try|typedef|typeid|typename|uint16_t|uint32_t|uint64_t|uint8_t|union|unsigned|using|",
                r"virtual|void|volatile|wchar_t|while)\b)\w+(?:\s*\.\s*\w+)*\b)",
            ],
            LB | GR,
            &ARDUINO_MODULE,
        )],
    ),
    e(
        "raw-string",
        &[pa(
            &[r#"R"([^()\\ ]{0,16})\([\s\S]*?\)\1""#],
            GR,
            &["string"],
        )],
    ),
    e(
        "string",
        &[pf(&[r#""(?:\\(?:\r\n|[\s\S])|[^"\\\r\n])*""#], GR)],
    ),
    e(
        "base-clause",
        &[pi(
            &[r#"(\b(?:class|struct)\s+\w+\s*:\s*)[^;{}"'\s]+(?:\s+[^;{}"'\s]+)*(?=\s*[;{])"#],
            LB | GR,
            &ARDUINO_BASE_CLAUSE,
        )],
    ),
    e(
        "class-name",
        &[
            pf(
                &[
                    r"(\b(?:class|concept|enum|struct|typename)\s+)(?!\b(?:alignas|alignof|asm|auto|bool|break|",
                    r"case|catch|char|char16_t|char32_t|char8_t|class|co_await|co_return|co_yield|compl|concept|",
                    r"const|const_cast|consteval|constexpr|constinit|continue|decltype|default|delete|do|double|",
                    r"dynamic_cast|else|enum|explicit|export|extern|final|float|for|friend|goto|if|import|inline|",
                    r"int|int16_t|int32_t|int64_t|int8_t|long|module|mutable|namespace|new|noexcept|nullptr|",
                    r"operator|override|private|protected|public|register|reinterpret_cast|requires|return|short|",
                    r"signed|sizeof|static|static_assert|static_cast|struct|switch|template|this|thread_local|",
                    r"throw|try|typedef|typeid|typename|uint16_t|uint32_t|uint64_t|uint8_t|union|unsigned|using|",
                    r"virtual|void|volatile|wchar_t|while)\b)\w+",
                ],
                LB,
            ),
            p(&[r"\b[A-Z]\w*(?=\s*::\s*\w+\s*\()"]),
            p(&[r"(?i)\b[A-Z_]\w*(?=\s*::\s*~\w+\s*\()"]),
            p(&[r"\b\w+(?=\s*<(?:[^<>]|<(?:[^<>]|<[^<>]*>)*>)*>\s*::\s*\w+\s*\()"]),
        ],
    ),
    e(
        "generic-function",
        &[pi(
            &[r"(?i)\b(?!operator\b)[a-z_]\w*\s*<(?:[^<>]|<[^<>]*>)*>(?=\s*\()"],
            0,
            &ARDUINO_GENERIC_FUNCTION,
        )],
    ),
    e(
        "keyword",
        &[p(&[
            r"\b(?:String|array|bool|boolean|break|byte|case|catch|continue|default|do|double|else|",
            r"finally|for|function|goto|if|in|instanceof|int|integer|long|loop|new|null|return|setup|",
            r"string|switch|throw|try|void|while|word)\b",
        ])],
    ),
    e(
        "constant",
        &[p(&[
            r"\b(?:ANALOG_MESSAGE|DEFAULT|DIGITAL_MESSAGE|EXTERNAL|FIRMATA_STRING|HIGH|INPUT|",
            r"INPUT_PULLUP|INTERNAL|INTERNAL1V1|INTERNAL2V56|LED_BUILTIN|LOW|OUTPUT|REPORT_ANALOG|",
            r"REPORT_DIGITAL|SET_PIN_MODE|SYSEX_START|SYSTEM_RESET)\b",
        ])],
    ),
    e("function", &[p(&[r"(?i)\b[a-z_]\w*(?=\s*\()"])]),
    e(
        "number",
        &[pf(
            &[
                r"(?i)(?:\b0b[01']+|\b0x(?:[\da-f']+(?:\.[\da-f']*)?|\.[\da-f']+)(?:p[+-]?[\d']+)?|",
                r"(?:\b[\d']+(?:\.[\d']*)?|\B\.[\d']+)(?:e[+-]?[\d']+)?)[ful]{0,4}",
            ],
            GR,
        )],
    ),
    e("double-colon", &[pa(&[r"::"], 0, &["punctuation"])]),
    e(
        "operator",
        &[p(&[
            r">>=?|<<=?|->|--|\+\+|&&|\|\||[?:\~]|<=>|[-+*/%\&|^!=<>]=?|\b(?:and|and_eq|bitand|bitor|not|",
            r"not_eq|or|or_eq|xor|xor_eq)\b",
        ])],
    ),
    e("punctuation", &[p(&[r"[{}\[\];(),.:]"])]),
    e("boolean", &[p(&[r"\b(?:false|true)\b"])]),
    e(
        "builtin",
        &[p(&[
            r"\b(?:Audio|BSSID|Bridge|Client|Console|EEPROM|Esplora|EsploraTFT|Ethernet|EthernetClient|",
            r"EthernetServer|EthernetUDP|File|FileIO|FileSystem|Firmata|GPRS|GSM|GSMBand|GSMClient|",
            r"GSMModem|GSMPIN|GSMScanner|GSMServer|GSMVoiceCall|GSM_SMS|HttpClient|IPAddress|IRread|",
            r"Keyboard|KeyboardController|LiquidCrystal|LiquidCrystal_I2C|Mailbox|Mouse|MouseController|",
            r"PImage|Process|RSSI|RobotControl|RobotMotor|SD|SPI|SSID|Scheduler|Serial|Server|Servo|",
            r"SoftwareSerial|Stepper|Stream|TFT|Task|USBHost|WiFi|WiFiClient|WiFiServer|WiFiUDP|Wire|",
            r"YunClient|YunServer|abs|addParameter|analogRead|analogReadResolution|analogReference|",
            r"analogWrite|analogWriteResolution|answerCall|attach|attachGPRS|attachInterrupt|attached|",
            r"autoscroll|available|background|beep|begin|beginPacket|beginSD|beginSMS|beginSpeaker|",
            r"beginTFT|beginTransmission|beginWrite|bit|bitClear|bitRead|bitSet|bitWrite|blink|",
            r"blinkVersion|buffer|changePIN|checkPIN|checkPUK|checkReg|circle|cityNameRead|cityNameWrite|",
            r"clear|clearScreen|click|close|compassRead|config|connect|connected|constrain|cos|",
            r"countryNameRead|countryNameWrite|createChar|cursor|debugPrint|delay|delayMicroseconds|",
            r"detach|detachInterrupt|digitalRead|digitalWrite|disconnect|display|displayLogos|drawBMP|",
            r"drawCompass|encryptionType|end|endPacket|endSMS|endTransmission|endWrite|exists|exitValue|",
            r"fill|find|findUntil|flush|gatewayIP|get|getAsynchronously|getBand|getButton|",
            r"getCurrentCarrier|getIMEI|getKey|getModifiers|getOemKey|getPINUsed|getResult|",
            r"getSignalStrength|getSocket|getVoiceCallStatus|getXChange|getYChange|hangCall|height|",
            r"highByte|home|image|interrupts|isActionDone|isDirectory|isListening|isPIN|isPressed|",
            r"isValid|keyPressed|keyReleased|keyboardRead|knobRead|leftToRight|line|lineFollowConfig|",
            r"listen|listenOnLocalhost|loadImage|localIP|lowByte|macAddress|maintain|map|max|",
            r"messageAvailable|micros|millis|min|mkdir|motorsStop|motorsWrite|mouseDragged|mouseMoved|",
            r"mousePressed|mouseReleased|move|noAutoscroll|noBlink|noBuffer|noCursor|noDisplay|noFill|",
            r"noInterrupts|noListenOnLocalhost|noStroke|noTone|onReceive|onRequest|open|openNextFile|",
            r"overflow|parseCommand|parseFloat|parseInt|parsePacket|pauseMode|peek|pinMode|playFile|",
            r"playMelody|point|pointTo|position|pow|prepare|press|print|printFirmwareVersion|",
            r"printVersion|println|process|processInput|pulseIn|put|random|randomSeed|read|",
            r"readAccelerometer|readBlue|readButton|readBytes|readBytesUntil|readGreen|",
            r"readJoystickButton|readJoystickSwitch|readJoystickX|readJoystickY|readLightSensor|",
            r"readMessage|readMicrophone|readNetworks|readRed|readSlider|readString|readStringUntil|",
            r"readTemperature|ready|rect|release|releaseAll|remoteIP|remoteNumber|remotePort|remove|",
            r"requestFrom|retrieveCallingNumber|rewindDirectory|rightToLeft|rmdir|robotNameRead|",
            r"robotNameWrite|run|runAsynchronously|runShellCommand|runShellCommandAsynchronously|running|",
            r"scanNetworks|scrollDisplayLeft|scrollDisplayRight|seek|sendAnalog|sendDigitalPortPair|",
            r"sendDigitalPorts|sendString|sendSysex|serialEvent|setBand|setBitOrder|setClockDivider|",
            r"setCursor|setDNS|setDataMode|setFirmwareVersion|setMode|setPINUsed|setSpeed|setTextSize|",
            r"setTimeout|shiftIn|shiftOut|shutdown|sin|size|sqrt|startLoop|step|stop|stroke|subnetMask|",
            r"switchPIN|tan|tempoWrite|text|tone|transfer|tuneWrite|turn|updateIR|userNameRead|",
            r"userNameWrite|voiceCall|waitContinue|width|write|writeBlue|writeGreen|writeJSON|",
            r"writeMessage|writeMicroseconds|writeRGB|writeRed|yield)\b",
        ])],
    ),
]);

pub(super) static ARDUINO_MACRO: Grammar = Grammar::new(&[
    e(
        "string",
        &[
            pf(&[r"^(#\s*include\s*)<[^>]+>"], LB),
            pf(&[r#""(?:\\(?:\r\n|[\s\S])|[^"\\\r\n])*""#], GR),
        ],
    ),
    e(
        "char",
        &[pf(&[r"'(?:\\(?:\r\n|[\s\S])|[^'\\\r\n]){0,32}'"], GR)],
    ),
    e(
        "comment",
        &[pf(
            &[r"//(?:[^\r\n\\]|\\(?:\r\n?|\n|(?![\r\n])))*|/\*[\s\S]*?(?:\*/|$)"],
            GR,
        )],
    ),
    e(
        "macro-name",
        &[
            pf(&[r"(?i)(^#\s*define\s+)\w+\b(?!\()"], LB),
            pa(&[r"(?i)(^#\s*define\s+)\w+\b(?=\()"], LB, &["function"]),
        ],
    ),
    e("directive", &[pa(&[r"^(#\s*)[a-z]+"], LB, &["keyword"])]),
    e("directive-hash", &[p(&[r"^#"])]),
    e("punctuation", &[p(&[r"##|\\(?=[\r\n])"])]),
    e("expression", &[pi(&[r"\S[\s\S]*"], 0, &ARDUINO)]),
]);

pub(super) static ARDUINO_MODULE: Grammar = Grammar::new(&[
    e("string", &[p(&[r#"^[<"][\s\S]+"#])]),
    e("operator", &[p(&[r":"])]),
    e("punctuation", &[p(&[r"\."])]),
]);

pub(super) static ARDUINO_BASE_CLAUSE: Grammar = Grammar::new(&[
    e(
        "comment",
        &[pf(
            &[r"//(?:[^\r\n\\]|\\(?:\r\n?|\n|(?![\r\n])))*|/\*[\s\S]*?(?:\*/|$)"],
            GR,
        )],
    ),
    e(
        "char",
        &[pf(&[r"'(?:\\(?:\r\n|[\s\S])|[^'\\\r\n]){0,32}'"], GR)],
    ),
    e(
        "macro",
        &[pai(
            &[
                r"(?im)(^[\t ]*)#\s*[a-z](?:[^\r\n\\/]|/(?!\*)|/\*(?:[^*]|\*(?!/))*\*/|\\(?:\r\n|[\s\S]))*",
            ],
            LB | GR,
            &["property"],
            &ARDUINO_BASE_CLAUSE_MACRO,
        )],
    ),
    e(
        "module",
        &[pi(
            &[
                r#"(\b(?:import|module)\s+)(?:"(?:\\(?:\r\n|[\s\S])|[^"\\\r\n])*"|<[^<>\r\n]*>|"#,
                r"\b(?!\b(?:alignas|alignof|asm|auto|bool|break|case|catch|char|char16_t|char32_t|char8_t|",
                r"class|co_await|co_return|co_yield|compl|concept|const|const_cast|consteval|constexpr|",
                r"constinit|continue|decltype|default|delete|do|double|dynamic_cast|else|enum|explicit|",
                r"export|extern|final|float|for|friend|goto|if|import|inline|int|int16_t|int32_t|int64_t|",
                r"int8_t|long|module|mutable|namespace|new|noexcept|nullptr|operator|override|private|",
                r"protected|public|register|reinterpret_cast|requires|return|short|signed|sizeof|static|",
                r"static_assert|static_cast|struct|switch|template|this|thread_local|throw|try|typedef|",
                r"typeid|typename|uint16_t|uint32_t|uint64_t|uint8_t|union|unsigned|using|virtual|void|",
                r"volatile|wchar_t|while)\b)\w+(?:\s*\.\s*\w+)*\b(?:\s*:\s*\b(?!\b(?:alignas|alignof|asm|",
                r"auto|bool|break|case|catch|char|char16_t|char32_t|char8_t|class|co_await|co_return|",
                r"co_yield|compl|concept|const|const_cast|consteval|constexpr|constinit|continue|decltype|",
                r"default|delete|do|double|dynamic_cast|else|enum|explicit|export|extern|final|float|for|",
                r"friend|goto|if|import|inline|int|int16_t|int32_t|int64_t|int8_t|long|module|mutable|",
                r"namespace|new|noexcept|nullptr|operator|override|private|protected|public|register|",
                r"reinterpret_cast|requires|return|short|signed|sizeof|static|static_assert|static_cast|",
                r"struct|switch|template|this|thread_local|throw|try|typedef|typeid|typename|uint16_t|",
                r"uint32_t|uint64_t|uint8_t|union|unsigned|using|virtual|void|volatile|wchar_t|",
                r"while)\b)\w+(?:\s*\.\s*\w+)*\b)?|:\s*\b(?!\b(?:alignas|alignof|asm|auto|bool|break|case|",
                r"catch|char|char16_t|char32_t|char8_t|class|co_await|co_return|co_yield|compl|concept|const|",
                r"const_cast|consteval|constexpr|constinit|continue|decltype|default|delete|do|double|",
                r"dynamic_cast|else|enum|explicit|export|extern|final|float|for|friend|goto|if|import|inline|",
                r"int|int16_t|int32_t|int64_t|int8_t|long|module|mutable|namespace|new|noexcept|nullptr|",
                r"operator|override|private|protected|public|register|reinterpret_cast|requires|return|short|",
                r"signed|sizeof|static|static_assert|static_cast|struct|switch|template|this|thread_local|",
                r"throw|try|typedef|typeid|typename|uint16_t|uint32_t|uint64_t|uint8_t|union|unsigned|using|",
                r"virtual|void|volatile|wchar_t|while)\b)\w+(?:\s*\.\s*\w+)*\b)",
            ],
            LB | GR,
            &ARDUINO_BASE_CLAUSE_MACRO_EXPRESSION_MODULE,
        )],
    ),
    e(
        "raw-string",
        &[pa(
            &[r#"R"([^()\\ ]{0,16})\([\s\S]*?\)\1""#],
            GR,
            &["string"],
        )],
    ),
    e(
        "string",
        &[pf(&[r#""(?:\\(?:\r\n|[\s\S])|[^"\\\r\n])*""#], GR)],
    ),
    e(
        "generic-function",
        &[pi(
            &[r"(?i)\b(?!operator\b)[a-z_]\w*\s*<(?:[^<>]|<[^<>]*>)*>(?=\s*\()"],
            0,
            &ARDUINO_BASE_CLAUSE_MACRO_EXPRESSION_GENERIC_FUNCTION,
        )],
    ),
    e(
        "keyword",
        &[p(&[
            r"\b(?:alignas|alignof|asm|auto|bool|break|case|catch|char|char16_t|char32_t|char8_t|class|",
            r"co_await|co_return|co_yield|compl|concept|const|const_cast|consteval|constexpr|constinit|",
            r"continue|decltype|default|delete|do|double|dynamic_cast|else|enum|explicit|export|extern|",
            r"final|float|for|friend|goto|if|import|inline|int|int16_t|int32_t|int64_t|int8_t|long|",
            r"module|mutable|namespace|new|noexcept|nullptr|operator|override|private|protected|public|",
            r"register|reinterpret_cast|requires|return|short|signed|sizeof|static|static_assert|",
            r"static_cast|struct|switch|template|this|thread_local|throw|try|typedef|typeid|typename|",
            r"uint16_t|uint32_t|uint64_t|uint8_t|union|unsigned|using|virtual|void|volatile|wchar_t|",
            r"while)\b",
        ])],
    ),
    e(
        "constant",
        &[p(&[
            r"\b(?:EOF|NULL|SEEK_CUR|SEEK_END|SEEK_SET|__DATE__|__FILE__|__LINE__|__TIMESTAMP__|__TIME__|",
            r"__func__|stderr|stdin|stdout)\b",
        ])],
    ),
    e("function", &[p(&[r"(?i)\b[a-z_]\w*(?=\s*\()"])]),
    e(
        "number",
        &[pf(
            &[
                r"(?i)(?:\b0b[01']+|\b0x(?:[\da-f']+(?:\.[\da-f']*)?|\.[\da-f']+)(?:p[+-]?[\d']+)?|",
                r"(?:\b[\d']+(?:\.[\d']*)?|\B\.[\d']+)(?:e[+-]?[\d']+)?)[ful]{0,4}",
            ],
            GR,
        )],
    ),
    e("class-name", &[p(&[r"(?i)\b[a-z_]\w*\b(?!\s*::)"])]),
    e("double-colon", &[pa(&[r"::"], 0, &["punctuation"])]),
    e(
        "operator",
        &[p(&[
            r">>=?|<<=?|->|--|\+\+|&&|\|\||[?:\~]|<=>|[-+*/%\&|^!=<>]=?|\b(?:and|and_eq|bitand|bitor|not|",
            r"not_eq|or|or_eq|xor|xor_eq)\b",
        ])],
    ),
    e("punctuation", &[p(&[r"[{}\[\];(),.:]"])]),
    e("boolean", &[p(&[r"\b(?:false|true)\b"])]),
]);

pub(super) static ARDUINO_BASE_CLAUSE_MACRO: Grammar = Grammar::new(&[
    e(
        "string",
        &[
            pf(&[r"^(#\s*include\s*)<[^>]+>"], LB),
            pf(&[r#""(?:\\(?:\r\n|[\s\S])|[^"\\\r\n])*""#], GR),
        ],
    ),
    e(
        "char",
        &[pf(&[r"'(?:\\(?:\r\n|[\s\S])|[^'\\\r\n]){0,32}'"], GR)],
    ),
    e(
        "comment",
        &[pf(
            &[r"//(?:[^\r\n\\]|\\(?:\r\n?|\n|(?![\r\n])))*|/\*[\s\S]*?(?:\*/|$)"],
            GR,
        )],
    ),
    e(
        "macro-name",
        &[
            pf(&[r"(?i)(^#\s*define\s+)\w+\b(?!\()"], LB),
            pa(&[r"(?i)(^#\s*define\s+)\w+\b(?=\()"], LB, &["function"]),
        ],
    ),
    e("directive", &[pa(&[r"^(#\s*)[a-z]+"], LB, &["keyword"])]),
    e("directive-hash", &[p(&[r"^#"])]),
    e("punctuation", &[p(&[r"##|\\(?=[\r\n])"])]),
    e(
        "expression",
        &[pi(
            &[r"\S[\s\S]*"],
            0,
            &ARDUINO_BASE_CLAUSE_MACRO_EXPRESSION,
        )],
    ),
]);

pub(super) static ARDUINO_BASE_CLAUSE_MACRO_EXPRESSION: Grammar = Grammar::new(&[
    e(
        "comment",
        &[pf(
            &[r"//(?:[^\r\n\\]|\\(?:\r\n?|\n|(?![\r\n])))*|/\*[\s\S]*?(?:\*/|$)"],
            GR,
        )],
    ),
    e(
        "char",
        &[pf(&[r"'(?:\\(?:\r\n|[\s\S])|[^'\\\r\n]){0,32}'"], GR)],
    ),
    e(
        "macro",
        &[pai(
            &[
                r"(?im)(^[\t ]*)#\s*[a-z](?:[^\r\n\\/]|/(?!\*)|/\*(?:[^*]|\*(?!/))*\*/|\\(?:\r\n|[\s\S]))*",
            ],
            LB | GR,
            &["property"],
            &ARDUINO_BASE_CLAUSE_MACRO,
        )],
    ),
    e(
        "module",
        &[pi(
            &[
                r#"(\b(?:import|module)\s+)(?:"(?:\\(?:\r\n|[\s\S])|[^"\\\r\n])*"|<[^<>\r\n]*>|"#,
                r"\b(?!\b(?:alignas|alignof|asm|auto|bool|break|case|catch|char|char16_t|char32_t|char8_t|",
                r"class|co_await|co_return|co_yield|compl|concept|const|const_cast|consteval|constexpr|",
                r"constinit|continue|decltype|default|delete|do|double|dynamic_cast|else|enum|explicit|",
                r"export|extern|final|float|for|friend|goto|if|import|inline|int|int16_t|int32_t|int64_t|",
                r"int8_t|long|module|mutable|namespace|new|noexcept|nullptr|operator|override|private|",
                r"protected|public|register|reinterpret_cast|requires|return|short|signed|sizeof|static|",
                r"static_assert|static_cast|struct|switch|template|this|thread_local|throw|try|typedef|",
                r"typeid|typename|uint16_t|uint32_t|uint64_t|uint8_t|union|unsigned|using|virtual|void|",
                r"volatile|wchar_t|while)\b)\w+(?:\s*\.\s*\w+)*\b(?:\s*:\s*\b(?!\b(?:alignas|alignof|asm|",
                r"auto|bool|break|case|catch|char|char16_t|char32_t|char8_t|class|co_await|co_return|",
                r"co_yield|compl|concept|const|const_cast|consteval|constexpr|constinit|continue|decltype|",
                r"default|delete|do|double|dynamic_cast|else|enum|explicit|export|extern|final|float|for|",
                r"friend|goto|if|import|inline|int|int16_t|int32_t|int64_t|int8_t|long|module|mutable|",
                r"namespace|new|noexcept|nullptr|operator|override|private|protected|public|register|",
                r"reinterpret_cast|requires|return|short|signed|sizeof|static|static_assert|static_cast|",
                r"struct|switch|template|this|thread_local|throw|try|typedef|typeid|typename|uint16_t|",
                r"uint32_t|uint64_t|uint8_t|union|unsigned|using|virtual|void|volatile|wchar_t|",
                r"while)\b)\w+(?:\s*\.\s*\w+)*\b)?|:\s*\b(?!\b(?:alignas|alignof|asm|auto|bool|break|case|",
                r"catch|char|char16_t|char32_t|char8_t|class|co_await|co_return|co_yield|compl|concept|const|",
                r"const_cast|consteval|constexpr|constinit|continue|decltype|default|delete|do|double|",
                r"dynamic_cast|else|enum|explicit|export|extern|final|float|for|friend|goto|if|import|inline|",
                r"int|int16_t|int32_t|int64_t|int8_t|long|module|mutable|namespace|new|noexcept|nullptr|",
                r"operator|override|private|protected|public|register|reinterpret_cast|requires|return|short|",
                r"signed|sizeof|static|static_assert|static_cast|struct|switch|template|this|thread_local|",
                r"throw|try|typedef|typeid|typename|uint16_t|uint32_t|uint64_t|uint8_t|union|unsigned|using|",
                r"virtual|void|volatile|wchar_t|while)\b)\w+(?:\s*\.\s*\w+)*\b)",
            ],
            LB | GR,
            &ARDUINO_BASE_CLAUSE_MACRO_EXPRESSION_MODULE,
        )],
    ),
    e(
        "raw-string",
        &[pa(
            &[r#"R"([^()\\ ]{0,16})\([\s\S]*?\)\1""#],
            GR,
            &["string"],
        )],
    ),
    e(
        "string",
        &[pf(&[r#""(?:\\(?:\r\n|[\s\S])|[^"\\\r\n])*""#], GR)],
    ),
    e(
        "class-name",
        &[
            pf(
                &[
                    r"(\b(?:class|concept|enum|struct|typename)\s+)(?!\b(?:alignas|alignof|asm|auto|bool|break|",
                    r"case|catch|char|char16_t|char32_t|char8_t|class|co_await|co_return|co_yield|compl|concept|",
                    r"const|const_cast|consteval|constexpr|constinit|continue|decltype|default|delete|do|double|",
                    r"dynamic_cast|else|enum|explicit|export|extern|final|float|for|friend|goto|if|import|inline|",
                    r"int|int16_t|int32_t|int64_t|int8_t|long|module|mutable|namespace|new|noexcept|nullptr|",
                    r"operator|override|private|protected|public|register|reinterpret_cast|requires|return|short|",
                    r"signed|sizeof|static|static_assert|static_cast|struct|switch|template|this|thread_local|",
                    r"throw|try|typedef|typeid|typename|uint16_t|uint32_t|uint64_t|uint8_t|union|unsigned|using|",
                    r"virtual|void|volatile|wchar_t|while)\b)\w+",
                ],
                LB,
            ),
            p(&[r"\b[A-Z]\w*(?=\s*::\s*\w+\s*\()"]),
            p(&[r"(?i)\b[A-Z_]\w*(?=\s*::\s*~\w+\s*\()"]),
            p(&[r"\b\w+(?=\s*<(?:[^<>]|<(?:[^<>]|<[^<>]*>)*>)*>\s*::\s*\w+\s*\()"]),
        ],
    ),
    e(
        "generic-function",
        &[pi(
            &[r"(?i)\b(?!operator\b)[a-z_]\w*\s*<(?:[^<>]|<[^<>]*>)*>(?=\s*\()"],
            0,
            &ARDUINO_BASE_CLAUSE_MACRO_EXPRESSION_GENERIC_FUNCTION,
        )],
    ),
    e(
        "keyword",
        &[p(&[
            r"\b(?:alignas|alignof|asm|auto|bool|break|case|catch|char|char16_t|char32_t|char8_t|class|",
            r"co_await|co_return|co_yield|compl|concept|const|const_cast|consteval|constexpr|constinit|",
            r"continue|decltype|default|delete|do|double|dynamic_cast|else|enum|explicit|export|extern|",
            r"final|float|for|friend|goto|if|import|inline|int|int16_t|int32_t|int64_t|int8_t|long|",
            r"module|mutable|namespace|new|noexcept|nullptr|operator|override|private|protected|public|",
            r"register|reinterpret_cast|requires|return|short|signed|sizeof|static|static_assert|",
            r"static_cast|struct|switch|template|this|thread_local|throw|try|typedef|typeid|typename|",
            r"uint16_t|uint32_t|uint64_t|uint8_t|union|unsigned|using|virtual|void|volatile|wchar_t|",
            r"while)\b",
        ])],
    ),
    e(
        "constant",
        &[p(&[
            r"\b(?:EOF|NULL|SEEK_CUR|SEEK_END|SEEK_SET|__DATE__|__FILE__|__LINE__|__TIMESTAMP__|__TIME__|",
            r"__func__|stderr|stdin|stdout)\b",
        ])],
    ),
    e("function", &[p(&[r"(?i)\b[a-z_]\w*(?=\s*\()"])]),
    e(
        "number",
        &[pf(
            &[
                r"(?i)(?:\b0b[01']+|\b0x(?:[\da-f']+(?:\.[\da-f']*)?|\.[\da-f']+)(?:p[+-]?[\d']+)?|",
                r"(?:\b[\d']+(?:\.[\d']*)?|\B\.[\d']+)(?:e[+-]?[\d']+)?)[ful]{0,4}",
            ],
            GR,
        )],
    ),
    e("double-colon", &[pa(&[r"::"], 0, &["punctuation"])]),
    e(
        "operator",
        &[p(&[
            r">>=?|<<=?|->|--|\+\+|&&|\|\||[?:\~]|<=>|[-+*/%\&|^!=<>]=?|\b(?:and|and_eq|bitand|bitor|not|",
            r"not_eq|or|or_eq|xor|xor_eq)\b",
        ])],
    ),
    e("punctuation", &[p(&[r"[{}\[\];(),.:]"])]),
    e("boolean", &[p(&[r"\b(?:false|true)\b"])]),
]);

pub(super) static ARDUINO_BASE_CLAUSE_MACRO_EXPRESSION_MODULE: Grammar = Grammar::new(&[
    e("string", &[p(&[r#"^[<"][\s\S]+"#])]),
    e("operator", &[p(&[r":"])]),
    e("punctuation", &[p(&[r"\."])]),
]);

pub(super) static ARDUINO_BASE_CLAUSE_MACRO_EXPRESSION_GENERIC_FUNCTION: Grammar = Grammar::new(&[
    e("function", &[p(&[r"^\w+"])]),
    e(
        "generic",
        &[pai(
            &[r"<[\s\S]+"],
            0,
            &["class-name"],
            &ARDUINO_BASE_CLAUSE_MACRO_EXPRESSION,
        )],
    ),
]);

pub(super) static ARDUINO_GENERIC_FUNCTION: Grammar = Grammar::new(&[
    e("function", &[p(&[r"^\w+"])]),
    e(
        "generic",
        &[pai(&[r"<[\s\S]+"], 0, &["class-name"], &ARDUINO)],
    ),
]);
