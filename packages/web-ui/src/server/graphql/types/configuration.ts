

export const configurationTypeDefs = `
  enum ConfigurationCategory {
    SYSTEM
    USER_INTERFACE
    AGENT
    WORKTREE
    MAPROOM
    SECURITY
    PERFORMANCE
    NOTIFICATION
    INTEGRATION
    EXPERIMENTAL
  }

  enum ConfigurationScope {
    GLOBAL
    USER
    SESSION
    WORKTREE
    AGENT
    REPOSITORY
  }

  enum ConfigurationDataType {
    STRING
    NUMBER
    BOOLEAN
    JSON
    ARRAY
    ENUM
  }

  type Configuration implements Node & Timestamped {
    id: ID!
    key: String!
    value: JSON!
    category: ConfigurationCategory!
    createdAt: DateTime!
    updatedAt: DateTime!
    
    # Extended configuration fields
    scope: ConfigurationScope!
    scopeId: String
    dataType: ConfigurationDataType!
    defaultValue: JSON
    description: String!
    displayName: String!
    isRequired: Boolean!
    isSecret: Boolean!
    isReadOnly: Boolean!
    validationSchema: JSON
    enumOptions: [String!]
    minValue: Float
    maxValue: Float
    pattern: String
    dependencies: [String!]!
    deprecatedAt: DateTime
    deprecationMessage: String
    version: String!
    lastModifiedBy: String
    tags: [String!]!
    
    # Computed fields
    isValid: Boolean!
    isDefault: Boolean!
    isDeprecated: Boolean!
    effectiveValue: JSON!
  }

  type ConfigurationConnection {
    edges: [ConfigurationEdge!]!
    pageInfo: PageInfo!
  }

  type ConfigurationEdge {
    node: Configuration!
    cursor: String!
  }

  # Configuration groups for easier management
  type ConfigurationGroup {
    category: ConfigurationCategory!
    displayName: String!
    description: String!
    configurations: [Configuration!]!
    totalCount: Int!
  }

  # Configuration validation
  type ConfigurationValidation {
    isValid: Boolean!
    errors: [ValidationError!]!
    warnings: [String!]!
  }

  # Configuration history for audit trail
  type ConfigurationHistory implements Node & Timestamped {
    id: ID!
    configurationId: ID!
    oldValue: JSON
    newValue: JSON!
    changedBy: String!
    reason: String
    createdAt: DateTime!
    updatedAt: DateTime!
    
    # Relations
    configuration: Configuration!
  }

  # Configuration template for quick setup
  type ConfigurationTemplate {
    id: ID!
    name: String!
    description: String!
    category: ConfigurationCategory!
    configurations: [ConfigurationTemplateItem!]!
    isBuiltIn: Boolean!
    version: String!
    author: String
    tags: [String!]!
  }

  type ConfigurationTemplateItem {
    key: String!
    value: JSON!
    description: String!
    required: Boolean!
  }

  # Input types
  input ConfigurationCreateInput {
    key: String!
    value: JSON!
    category: ConfigurationCategory!
    scope: ConfigurationScope = GLOBAL
    scopeId: String
    dataType: ConfigurationDataType!
    description: String!
    displayName: String!
    isRequired: Boolean = false
    isSecret: Boolean = false
    validationSchema: JSON
    enumOptions: [String!]
    minValue: Float
    maxValue: Float
    pattern: String
    dependencies: [String!]
    tags: [String!]
  }

  input ConfigurationUpdateInput {
    id: ID!
    value: JSON
    description: String
    displayName: String
    isRequired: Boolean
    validationSchema: JSON
    enumOptions: [String!]
    minValue: Float
    maxValue: Float
    pattern: String
    dependencies: [String!]
    tags: [String!]
    reason: String
  }

  input ConfigurationFilterInput {
    category: ConfigurationCategory
    scope: ConfigurationScope
    scopeId: String
    isRequired: Boolean
    isSecret: Boolean
    isReadOnly: Boolean
    isDeprecated: Boolean
    tags: [String!]
    search: String
    keys: [String!]
  }

  input ConfigurationBulkUpdateInput {
    updates: [ConfigurationUpdateInput!]!
    reason: String
  }

  input ConfigurationTemplateApplyInput {
    templateId: ID!
    scope: ConfigurationScope = GLOBAL
    scopeId: String
    overrideExisting: Boolean = false
    values: [ConfigurationValueOverride!]
  }

  input ConfigurationValueOverride {
    key: String!
    value: JSON!
  }

  # Response types
  type ConfigurationResponse implements Response {
    success: Boolean!
    errors: [Error!]
    configuration: Configuration
  }

  type ConfigurationDeleteResponse implements Response {
    success: Boolean!
    errors: [Error!]
    deletedId: ID
  }

  type ConfigurationBulkUpdateResponse implements Response {
    success: Boolean!
    errors: [Error!]
    updatedConfigurations: [Configuration!]
    failedUpdates: [ConfigurationUpdateError!]
  }

  type ConfigurationUpdateError {
    input: ConfigurationUpdateInput!
    errors: [Error!]!
  }

  type ConfigurationTemplateApplyResponse implements Response {
    success: Boolean!
    errors: [Error!]
    appliedConfigurations: [Configuration!]
    skippedConfigurations: [String!]
  }

  extend type Query {
    configuration(id: ID!): Configuration
    configurations(
      filter: ConfigurationFilterInput
      sort: SortInput
      pagination: PaginationInput
    ): ConfigurationConnection!
    configurationByKey(key: String!, scope: ConfigurationScope = GLOBAL, scopeId: String): Configuration
    configurationGroups(scope: ConfigurationScope = GLOBAL, scopeId: String): [ConfigurationGroup!]!
    configurationHistory(
      configurationId: ID!
      pagination: PaginationInput
    ): [ConfigurationHistory!]!
    
    # Validation and templates
    validateConfiguration(key: String!, value: JSON!): ConfigurationValidation!
    validateConfigurations(inputs: [ConfigurationCreateInput!]!): [ConfigurationValidation!]!
    configurationTemplates(category: ConfigurationCategory): [ConfigurationTemplate!]!
    configurationTemplate(id: ID!): ConfigurationTemplate
    
    # Utility queries
    configurationDefaults(category: ConfigurationCategory): [Configuration!]!
    requiredConfigurations(scope: ConfigurationScope = GLOBAL, scopeId: String): [Configuration!]!
    deprecatedConfigurations: [Configuration!]!
  }

  extend type Mutation {
    createConfiguration(input: ConfigurationCreateInput!): ConfigurationResponse!
    updateConfiguration(input: ConfigurationUpdateInput!): ConfigurationResponse!
    deleteConfiguration(id: ID!): ConfigurationDeleteResponse!
    bulkUpdateConfigurations(input: ConfigurationBulkUpdateInput!): ConfigurationBulkUpdateResponse!
    
    # Value management
    setConfigurationValue(key: String!, value: JSON!, scope: ConfigurationScope = GLOBAL, scopeId: String, reason: String): ConfigurationResponse!
    resetConfigurationToDefault(id: ID!, reason: String): ConfigurationResponse!
    resetConfigurationsToDefaults(category: ConfigurationCategory, scope: ConfigurationScope = GLOBAL, scopeId: String, reason: String): ConfigurationBulkUpdateResponse!
    
    # Template operations
    applyConfigurationTemplate(input: ConfigurationTemplateApplyInput!): ConfigurationTemplateApplyResponse!
    createConfigurationTemplate(name: String!, description: String!, category: ConfigurationCategory!, configurations: [String!]!): ConfigurationTemplate!
    
    # Maintenance operations
    markConfigurationDeprecated(id: ID!, message: String!, replacementKey: String): ConfigurationResponse!
    migrateDeprecatedConfigurations: ConfigurationBulkUpdateResponse!
    cleanupDeprecatedConfigurations(olderThan: DateTime): Int!
  }

  extend type Subscription {
    configurationUpdated(id: ID): Configuration!
    configurationChanged(key: String, scope: ConfigurationScope, scopeId: String): Configuration!
    configurationCreated(category: ConfigurationCategory): Configuration!
    configurationDeleted: ID!
  }
`;