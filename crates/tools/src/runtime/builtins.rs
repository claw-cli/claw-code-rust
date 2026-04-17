use std::sync::Arc;

use crate::runtime::{
    LegacyRuntimeToolAdapter, RuntimeToolRegistry, ShellCommandRuntimeTool, ToolCapabilityTag,
    ToolDefinitionSpec, ToolExecutionMode, ToolName, ToolOutputMode,
};
use crate::{
    ApplyPatchTool, BashTool, FileWriteTool, GlobTool, GrepTool, InvalidTool, LspTool, PlanTool,
    QuestionTool, ReadTool, SkillTool, TaskTool, TodoWriteTool, Tool, WebFetchTool, WebSearchTool,
};

pub fn register_builtin_runtime_tools(registry: &RuntimeToolRegistry) {
    registry.register(Arc::new(ShellCommandRuntimeTool));
    register_legacy_builtin(
        registry,
        Arc::new(BashTool),
        runtime_definition(
            "bash",
            BashTool.description(),
            BashTool.input_schema(),
            ToolOutputMode::Mixed,
            ToolExecutionMode::Mutating,
            vec![ToolCapabilityTag::ExecuteProcess],
        ),
    );
    register_legacy_builtin(
        registry,
        Arc::new(ReadTool),
        runtime_definition(
            "read",
            ReadTool.description(),
            ReadTool.input_schema(),
            ToolOutputMode::Mixed,
            ToolExecutionMode::ReadOnly,
            vec![ToolCapabilityTag::ReadFiles],
        ),
    );
    register_legacy_builtin(
        registry,
        Arc::new(FileWriteTool),
        runtime_definition(
            "write",
            FileWriteTool.description(),
            FileWriteTool.input_schema(),
            ToolOutputMode::Mixed,
            ToolExecutionMode::Mutating,
            vec![ToolCapabilityTag::WriteFiles],
        ),
    );
    register_legacy_builtin(
        registry,
        Arc::new(GlobTool),
        runtime_definition(
            "glob",
            GlobTool.description(),
            GlobTool.input_schema(),
            ToolOutputMode::Text,
            ToolExecutionMode::ReadOnly,
            vec![ToolCapabilityTag::SearchWorkspace],
        ),
    );
    register_legacy_builtin(
        registry,
        Arc::new(GrepTool),
        runtime_definition(
            "grep",
            GrepTool.description(),
            GrepTool.input_schema(),
            ToolOutputMode::Text,
            ToolExecutionMode::ReadOnly,
            vec![ToolCapabilityTag::SearchWorkspace],
        ),
    );
    register_legacy_builtin(
        registry,
        Arc::new(InvalidTool),
        runtime_definition(
            "invalid",
            InvalidTool.description(),
            InvalidTool.input_schema(),
            ToolOutputMode::Text,
            ToolExecutionMode::ReadOnly,
            Vec::new(),
        ),
    );
    register_legacy_builtin(
        registry,
        Arc::new(QuestionTool),
        runtime_definition(
            "question",
            QuestionTool.description(),
            QuestionTool.input_schema(),
            ToolOutputMode::Text,
            ToolExecutionMode::ReadOnly,
            Vec::new(),
        ),
    );
    register_legacy_builtin(
        registry,
        Arc::new(TaskTool),
        runtime_definition(
            "task",
            TaskTool.description(),
            TaskTool.input_schema(),
            ToolOutputMode::Text,
            ToolExecutionMode::Mutating,
            Vec::new(),
        ),
    );
    register_legacy_builtin(
        registry,
        Arc::new(TodoWriteTool),
        runtime_definition(
            "todowrite",
            TodoWriteTool.description(),
            TodoWriteTool.input_schema(),
            ToolOutputMode::Text,
            ToolExecutionMode::Mutating,
            vec![ToolCapabilityTag::WriteFiles],
        ),
    );
    register_legacy_builtin(
        registry,
        Arc::new(WebFetchTool),
        runtime_definition(
            "webfetch",
            WebFetchTool.description(),
            WebFetchTool.input_schema(),
            ToolOutputMode::Mixed,
            ToolExecutionMode::ReadOnly,
            vec![ToolCapabilityTag::NetworkAccess],
        ),
    );
    register_legacy_builtin(
        registry,
        Arc::new(WebSearchTool),
        runtime_definition(
            "websearch",
            WebSearchTool.description(),
            WebSearchTool.input_schema(),
            ToolOutputMode::Text,
            ToolExecutionMode::ReadOnly,
            vec![ToolCapabilityTag::NetworkAccess],
        ),
    );
    register_legacy_builtin(
        registry,
        Arc::new(SkillTool),
        runtime_definition(
            "skill",
            SkillTool.description(),
            SkillTool.input_schema(),
            ToolOutputMode::Text,
            ToolExecutionMode::ReadOnly,
            Vec::new(),
        ),
    );
    register_legacy_builtin(
        registry,
        Arc::new(ApplyPatchTool),
        runtime_definition(
            "apply_patch",
            ApplyPatchTool.description(),
            ApplyPatchTool.input_schema(),
            ToolOutputMode::Mixed,
            ToolExecutionMode::Mutating,
            vec![ToolCapabilityTag::WriteFiles],
        ),
    );
    register_legacy_builtin(
        registry,
        Arc::new(LspTool),
        runtime_definition(
            "lsp",
            LspTool.description(),
            LspTool.input_schema(),
            ToolOutputMode::Text,
            ToolExecutionMode::ReadOnly,
            vec![ToolCapabilityTag::SearchWorkspace],
        ),
    );
    register_legacy_builtin(
        registry,
        Arc::new(PlanTool),
        runtime_definition(
            "update_plan",
            PlanTool.description(),
            PlanTool.input_schema(),
            ToolOutputMode::Text,
            ToolExecutionMode::Mutating,
            Vec::new(),
        ),
    );
}

fn register_legacy_builtin(
    registry: &RuntimeToolRegistry,
    tool: Arc<dyn crate::Tool>,
    definition: ToolDefinitionSpec,
) {
    registry.register(Arc::new(LegacyRuntimeToolAdapter::new(tool, definition)));
}

fn runtime_definition(
    name: &str,
    description: &str,
    input_schema: serde_json::Value,
    output_mode: ToolOutputMode,
    execution_mode: ToolExecutionMode,
    capability_tags: Vec<ToolCapabilityTag>,
) -> ToolDefinitionSpec {
    ToolDefinitionSpec {
        name: ToolName(name.into()),
        description: description.to_string(),
        input_schema,
        output_mode,
        execution_mode,
        capability_tags,
    }
}
