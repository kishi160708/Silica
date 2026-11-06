//! C++ (`cilk`, `cilk-cpp`, `cilkcpp`).
//!
//! Ported from Prism 1.29.0's `prism-clike.js`, `prism-c.js`, `prism-cpp.js` and `prism-opencl.js`,
//! as the grammar stands once all of them have loaded. Written by
//! `tools/prism-port/port.js`, which walks the finished grammar objects;
//! change that script and re-run it rather than editing this file.

use crate::grammar::{e, p, pa, pai, pf, pi, Grammar, GR, LB};

pub static CPP: Grammar = Grammar::new(&[
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
            &CPP_MACRO,
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
            &CPP_MODULE,
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
            &CPP_BASE_CLAUSE,
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
            &CPP_GENERIC_FUNCTION,
        )],
    ),
    e(
        "type-opencl-host",
        &[pa(
            &[
                r"\b(?:cl_(?:GLenum|GLint|GLuin|addressing_mode|bitfield|bool|buffer_create_type|",
                r"build_status|channel_(?:order|type)|(?:u?(?:char|int|long|short)|double|float)(?:2|3|4|8|",
                r"16)?|command_(?:queue(?:_info|_properties)?|type)|context(?:_info|_properties)?|",
                r"device_(?:exec_capabilities|fp_config|id|info|local_mem_type|mem_cache_type|type)|(?:event|",
                r"sampler)(?:_info)?|filter_mode|half|image_info|kernel(?:_info|_work_group_info)?|map_flags|",
                r"mem(?:_flags|_info|_object_type)?|platform_(?:id|info)|profiling_info|",
                r"program(?:_build_info|_info)?))\b",
            ],
            0,
            &["keyword"],
        )],
    ),
    e(
        "boolean-opencl-host",
        &[pa(&[r"\bCL_(?:FALSE|TRUE)\b"], 0, &["boolean"])],
    ),
    e(
        "constant-opencl-host",
        &[pa(
            &[
                r"\bCL_(?:A|ABGR|ADDRESS_(?:CLAMP(?:_TO_EDGE)?|MIRRORED_REPEAT|NONE|REPEAT)|ARGB|BGRA|",
                r"BLOCKING|BUFFER_CREATE_TYPE_REGION|BUILD_(?:ERROR|IN_PROGRESS|NONE|PROGRAM_FAILURE|",
                r"SUCCESS)|COMMAND_(?:ACQUIRE_GL_OBJECTS|BARRIER|COPY_(?:BUFFER(?:_RECT|_TO_IMAGE)?|",
                r"IMAGE(?:_TO_BUFFER)?)|FILL_(?:BUFFER|IMAGE)|MAP(?:_BUFFER|_IMAGE)|MARKER|",
                r"MIGRATE(?:_SVM)?_MEM_OBJECTS|NATIVE_KERNEL|NDRANGE_KERNEL|READ_(?:BUFFER(?:_RECT)?|IMAGE)|",
                r"RELEASE_GL_OBJECTS|SVM_(?:FREE|MAP|MEMCPY|MEMFILL|UNMAP)|TASK|UNMAP_MEM_OBJECT|USER|",
                r"WRITE_(?:BUFFER(?:_RECT)?|IMAGE))|COMPILER_NOT_AVAILABLE|COMPILE_PROGRAM_FAILURE|COMPLETE|",
                r"CONTEXT_(?:DEVICES|INTEROP_USER_SYNC|NUM_DEVICES|PLATFORM|PROPERTIES|REFERENCE_COUNT)|",
                r"DEPTH(?:_STENCIL)?|DEVICE_(?:ADDRESS_BITS|AFFINITY_DOMAIN_(?:L[1-4]_CACHE|",
                r"NEXT_PARTITIONABLE|NUMA)|AVAILABLE|BUILT_IN_KERNELS|COMPILER_AVAILABLE|DOUBLE_FP_CONFIG|",
                r"ENDIAN_LITTLE|ERROR_CORRECTION_SUPPORT|EXECUTION_CAPABILITIES|EXTENSIONS|",
                r"GLOBAL_(?:MEM_(?:CACHELINE_SIZE|CACHE_SIZE|CACHE_TYPE|SIZE)|VARIABLE_PREFERRED_TOTAL_SIZE)|",
                r"HOST_UNIFIED_MEMORY|IL_VERSION|IMAGE(?:2D_MAX_(?:HEIGHT|WIDTH)|3D_MAX_(?:DEPTH|HEIGHT|",
                r"WIDTH)|_BASE_ADDRESS_ALIGNMENT|_MAX_ARRAY_SIZE|_MAX_BUFFER_SIZE|_PITCH_ALIGNMENT|_SUPPORT)|",
                r"LINKER_AVAILABLE|LOCAL_MEM_SIZE|LOCAL_MEM_TYPE|MAX_(?:CLOCK_FREQUENCY|COMPUTE_UNITS|",
                r"CONSTANT_ARGS|CONSTANT_BUFFER_SIZE|GLOBAL_VARIABLE_SIZE|MEM_ALLOC_SIZE|NUM_SUB_GROUPS|",
                r"ON_DEVICE_(?:EVENTS|QUEUES)|PARAMETER_SIZE|PIPE_ARGS|READ_IMAGE_ARGS|READ_WRITE_IMAGE_ARGS|",
                r"SAMPLERS|WORK_GROUP_SIZE|WORK_ITEM_DIMENSIONS|WORK_ITEM_SIZES|WRITE_IMAGE_ARGS)|",
                r"MEM_BASE_ADDR_ALIGN|MIN_DATA_TYPE_ALIGN_SIZE|NAME|NATIVE_VECTOR_WIDTH_(?:CHAR|DOUBLE|FLOAT|",
                r"HALF|INT|LONG|SHORT)|NOT_(?:AVAILABLE|FOUND)|OPENCL_C_VERSION|PARENT_DEVICE|",
                r"PARTITION_(?:AFFINITY_DOMAIN|BY_AFFINITY_DOMAIN|BY_COUNTS|BY_COUNTS_LIST_END|EQUALLY|",
                r"FAILED|MAX_SUB_DEVICES|PROPERTIES|TYPE)|PIPE_MAX_(?:ACTIVE_RESERVATIONS|PACKET_SIZE)|",
                r"PLATFORM|PREFERRED_(?:GLOBAL_ATOMIC_ALIGNMENT|INTEROP_USER_SYNC|LOCAL_ATOMIC_ALIGNMENT|",
                r"PLATFORM_ATOMIC_ALIGNMENT|VECTOR_WIDTH_(?:CHAR|DOUBLE|FLOAT|HALF|INT|LONG|SHORT))|",
                r"PRINTF_BUFFER_SIZE|PROFILE|PROFILING_TIMER_RESOLUTION|QUEUE_(?:ON_(?:DEVICE_(?:MAX_SIZE|",
                r"PREFERRED_SIZE|PROPERTIES)|HOST_PROPERTIES)|PROPERTIES)|REFERENCE_COUNT|SINGLE_FP_CONFIG|",
                r"SUB_GROUP_INDEPENDENT_FORWARD_PROGRESS|SVM_(?:ATOMICS|CAPABILITIES|COARSE_GRAIN_BUFFER|",
                r"FINE_GRAIN_BUFFER|FINE_GRAIN_SYSTEM)|TYPE(?:_ACCELERATOR|_ALL|_CPU|_CUSTOM|_DEFAULT|_GPU)?|",
                r"VENDOR(?:_ID)?|VERSION)|DRIVER_VERSION|EVENT_(?:COMMAND_(?:EXECUTION_STATUS|QUEUE|TYPE)|",
                r"CONTEXT|REFERENCE_COUNT)|EXEC_(?:KERNEL|NATIVE_KERNEL|",
                r"STATUS_ERROR_FOR_EVENTS_IN_WAIT_LIST)|FILTER_(?:LINEAR|NEAREST)|FLOAT|",
                r"FP_(?:CORRECTLY_ROUNDED_DIVIDE_SQRT|DENORM|FMA|INF_NAN|ROUND_TO_INF|ROUND_TO_NEAREST|",
                r"ROUND_TO_ZERO|SOFT_FLOAT)|GLOBAL|HALF_FLOAT|IMAGE_(?:ARRAY_SIZE|BUFFER|DEPTH|ELEMENT_SIZE|",
                r"FORMAT|FORMAT_MISMATCH|FORMAT_NOT_SUPPORTED|HEIGHT|NUM_MIP_LEVELS|NUM_SAMPLES|ROW_PITCH|",
                r"SLICE_PITCH|WIDTH)|INTENSITY|INVALID_(?:ARG_INDEX|ARG_SIZE|ARG_VALUE|BINARY|BUFFER_SIZE|",
                r"BUILD_OPTIONS|COMMAND_QUEUE|COMPILER_OPTIONS|CONTEXT|DEVICE|DEVICE_PARTITION_COUNT|",
                r"DEVICE_QUEUE|DEVICE_TYPE|EVENT|EVENT_WAIT_LIST|GLOBAL_OFFSET|GLOBAL_WORK_SIZE|GL_OBJECT|",
                r"HOST_PTR|IMAGE_DESCRIPTOR|IMAGE_FORMAT_DESCRIPTOR|IMAGE_SIZE|KERNEL|KERNEL_ARGS|",
                r"KERNEL_DEFINITION|KERNEL_NAME|LINKER_OPTIONS|MEM_OBJECT|MIP_LEVEL|OPERATION|PIPE_SIZE|",
                r"PLATFORM|PROGRAM|PROGRAM_EXECUTABLE|PROPERTY|QUEUE_PROPERTIES|SAMPLER|VALUE|WORK_DIMENSION|",
                r"WORK_GROUP_SIZE|WORK_ITEM_SIZE)|KERNEL_(?:ARG_(?:ACCESS_(?:NONE|QUALIFIER|READ_ONLY|",
                r"READ_WRITE|WRITE_ONLY)|ADDRESS_(?:CONSTANT|GLOBAL|LOCAL|PRIVATE|QUALIFIER)|",
                r"INFO_NOT_AVAILABLE|NAME|TYPE_(?:CONST|NAME|NONE|PIPE|QUALIFIER|RESTRICT|VOLATILE))|",
                r"ATTRIBUTES|COMPILE_NUM_SUB_GROUPS|COMPILE_WORK_GROUP_SIZE|CONTEXT|",
                r"EXEC_INFO_SVM_FINE_GRAIN_SYSTEM|EXEC_INFO_SVM_PTRS|FUNCTION_NAME|GLOBAL_WORK_SIZE|",
                r"LOCAL_MEM_SIZE|LOCAL_SIZE_FOR_SUB_GROUP_COUNT|MAX_NUM_SUB_GROUPS|",
                r"MAX_SUB_GROUP_SIZE_FOR_NDRANGE|NUM_ARGS|PREFERRED_WORK_GROUP_SIZE_MULTIPLE|",
                r"PRIVATE_MEM_SIZE|PROGRAM|REFERENCE_COUNT|SUB_GROUP_COUNT_FOR_NDRANGE|WORK_GROUP_SIZE)|",
                r"LINKER_NOT_AVAILABLE|LINK_PROGRAM_FAILURE|LOCAL|LUMINANCE|MAP_(?:FAILURE|READ|WRITE|",
                r"WRITE_INVALIDATE_REGION)|MEM_(?:ALLOC_HOST_PTR|ASSOCIATED_MEMOBJECT|CONTEXT|COPY_HOST_PTR|",
                r"COPY_OVERLAP|FLAGS|HOST_NO_ACCESS|HOST_PTR|HOST_READ_ONLY|HOST_WRITE_ONLY|",
                r"KERNEL_READ_AND_WRITE|MAP_COUNT|OBJECT_(?:ALLOCATION_FAILURE|BUFFER|IMAGE1D|IMAGE1D_ARRAY|",
                r"IMAGE1D_BUFFER|IMAGE2D|IMAGE2D_ARRAY|IMAGE3D|PIPE)|OFFSET|READ_ONLY|READ_WRITE|",
                r"REFERENCE_COUNT|SIZE|SVM_ATOMICS|SVM_FINE_GRAIN_BUFFER|TYPE|USES_SVM_POINTER|USE_HOST_PTR|",
                r"WRITE_ONLY)|MIGRATE_MEM_OBJECT_(?:CONTENT_UNDEFINED|HOST)|MISALIGNED_SUB_BUFFER_OFFSET|",
                r"NONE|NON_BLOCKING|OUT_OF_(?:HOST_MEMORY|RESOURCES)|PIPE_(?:MAX_PACKETS|PACKET_SIZE)|",
                r"PLATFORM_(?:EXTENSIONS|HOST_TIMER_RESOLUTION|NAME|PROFILE|VENDOR|VERSION)|",
                r"PROFILING_(?:COMMAND_(?:COMPLETE|END|QUEUED|START|SUBMIT)|INFO_NOT_AVAILABLE)|",
                r"PROGRAM_(?:BINARIES|BINARY_SIZES|BINARY_TYPE(?:_COMPILED_OBJECT|_EXECUTABLE|_LIBRARY|",
                r"_NONE)?|BUILD_(?:GLOBAL_VARIABLE_TOTAL_SIZE|LOG|OPTIONS|STATUS)|CONTEXT|DEVICES|IL|",
                r"KERNEL_NAMES|NUM_DEVICES|NUM_KERNELS|REFERENCE_COUNT|SOURCE)|QUEUED|QUEUE_(?:CONTEXT|",
                r"DEVICE|DEVICE_DEFAULT|ON_DEVICE|ON_DEVICE_DEFAULT|OUT_OF_ORDER_EXEC_MODE_ENABLE|",
                r"PROFILING_ENABLE|PROPERTIES|REFERENCE_COUNT|SIZE)|R|RA|READ_(?:ONLY|WRITE)_CACHE|RG|RGB|",
                r"RGBA|RGBx|RGx|RUNNING|Rx|SAMPLER_(?:ADDRESSING_MODE|CONTEXT|FILTER_MODE|LOD_MAX|LOD_MIN|",
                r"MIP_FILTER_MODE|NORMALIZED_COORDS|REFERENCE_COUNT)|(?:UN)?SIGNED_INT(?:8|16|32)|",
                r"SNORM_INT(?:8|16)|SUBMITTED|SUCCESS|UNORM_INT(?:8|16|24|_101010|_101010_2)|",
                r"UNORM_SHORT_(?:555|565)|VERSION_(?:1_0|1_1|1_2|2_0|2_1)|sBGRA|sRGB|sRGBA|sRGBx)\b",
            ],
            0,
            &["constant"],
        )],
    ),
    e(
        "function-opencl-host",
        &[pa(
            &[
                r"\bcl(?:BuildProgram|CloneKernel|CompileProgram|Create(?:Buffer|",
                r"CommandQueue(?:WithProperties)?|Context|ContextFromType|Image|Image2D|Image3D|Kernel|",
                r"KernelsInProgram|Pipe|ProgramWith(?:Binary|BuiltInKernels|IL|Source)|Sampler|",
                r"SamplerWithProperties|SubBuffer|SubDevices|UserEvent)|Enqueue(?:(?:Barrier|",
                r"Marker)(?:WithWaitList)?|Copy(?:Buffer(?:Rect|ToImage)?|Image(?:ToBuffer)?)|(?:Fill|",
                r"Map)(?:Buffer|Image)|MigrateMemObjects|NDRangeKernel|NativeKernel|(?:Read|",
                r"Write)(?:Buffer(?:Rect)?|Image)|SVM(?:Free|Map|MemFill|Memcpy|MigrateMem|Unmap)|Task|",
                r"UnmapMemObject|WaitForEvents)|Finish|Flush|Get(?:CommandQueueInfo|ContextInfo|",
                r"Device(?:AndHostTimer|IDs|Info)|Event(?:Profiling)?Info|",
                r"ExtensionFunctionAddress(?:ForPlatform)?|HostTimer|ImageInfo|Kernel(?:ArgInfo|Info|",
                r"SubGroupInfo|WorkGroupInfo)|MemObjectInfo|PipeInfo|Platform(?:IDs|Info)|",
                r"Program(?:Build)?Info|SamplerInfo|SupportedImageFormats)|LinkProgram|(?:Release|",
                r"Retain)(?:CommandQueue|Context|Device|Event|Kernel|MemObject|Program|Sampler)|SVM(?:Alloc|",
                r"Free)|Set(?:CommandQueueProperty|DefaultDeviceCommandQueue|EventCallback|Kernel|",
                r"Kernel(?:Arg(?:SVMPointer)?|ExecInfo)|MemObjectDestructorCallback|UserEventStatus)|",
                r"Unload(?:Platform)?Compiler|WaitForEvents)\b",
            ],
            0,
            &["function"],
        )],
    ),
    e(
        "type-opencl-host-cpp",
        &[pa(
            &[
                r"\b(?:Buffer|BufferGL|BufferRenderGL|CommandQueue|Context|Device|DeviceCommandQueue|",
                r"EnqueueArgs|Event|Image|Image1D|Image1DArray|Image1DBuffer|Image2D|Image2DArray|Image2DGL|",
                r"Image3D|Image3DGL|ImageFormat|ImageGL|Kernel|KernelFunctor|LocalSpaceArg|Memory|NDRange|",
                r"Pipe|Platform|Program|SVMAllocator|SVMTraitAtomic|SVMTraitCoarse|SVMTraitFine|",
                r"SVMTraitReadOnly|SVMTraitReadWrite|SVMTraitWriteOnly|Sampler|UserEvent)\b",
            ],
            0,
            &["keyword"],
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
    e(
        "parallel-keyword",
        &[pa(
            &[r"\bcilk_(?:for|reducer|s(?:cope|pawn|ync))\b"],
            0,
            &["keyword"],
        )],
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

pub(super) static CPP_MACRO: Grammar = Grammar::new(&[
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
    e("expression", &[pi(&[r"\S[\s\S]*"], 0, &CPP)]),
]);

pub(super) static CPP_MODULE: Grammar = Grammar::new(&[
    e("string", &[p(&[r#"^[<"][\s\S]+"#])]),
    e("operator", &[p(&[r":"])]),
    e("punctuation", &[p(&[r"\."])]),
]);

pub(super) static CPP_BASE_CLAUSE: Grammar = Grammar::new(&[
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
            &CPP_BASE_CLAUSE_MACRO,
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
            &CPP_BASE_CLAUSE_MACRO_EXPRESSION_MODULE,
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
            &CPP_BASE_CLAUSE_MACRO_EXPRESSION_GENERIC_FUNCTION,
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

pub(super) static CPP_BASE_CLAUSE_MACRO: Grammar = Grammar::new(&[
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
        &[pi(&[r"\S[\s\S]*"], 0, &CPP_BASE_CLAUSE_MACRO_EXPRESSION)],
    ),
]);

pub(super) static CPP_BASE_CLAUSE_MACRO_EXPRESSION: Grammar = Grammar::new(&[
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
            &CPP_BASE_CLAUSE_MACRO,
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
            &CPP_BASE_CLAUSE_MACRO_EXPRESSION_MODULE,
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
            &CPP_BASE_CLAUSE_MACRO_EXPRESSION_GENERIC_FUNCTION,
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

pub(super) static CPP_BASE_CLAUSE_MACRO_EXPRESSION_MODULE: Grammar = Grammar::new(&[
    e("string", &[p(&[r#"^[<"][\s\S]+"#])]),
    e("operator", &[p(&[r":"])]),
    e("punctuation", &[p(&[r"\."])]),
]);

pub(super) static CPP_BASE_CLAUSE_MACRO_EXPRESSION_GENERIC_FUNCTION: Grammar = Grammar::new(&[
    e("function", &[p(&[r"^\w+"])]),
    e(
        "generic",
        &[pai(
            &[r"<[\s\S]+"],
            0,
            &["class-name"],
            &CPP_BASE_CLAUSE_MACRO_EXPRESSION,
        )],
    ),
]);

pub(super) static CPP_GENERIC_FUNCTION: Grammar = Grammar::new(&[
    e("function", &[p(&[r"^\w+"])]),
    e("generic", &[pai(&[r"<[\s\S]+"], 0, &["class-name"], &CPP)]),
]);
