# Mock Responses Support Analysis & Enhancement Recommendations

Based on my analysis of the codebase, here are all potential enhancements for the Mock Responses feature, sorted by **descending priority**:

## **Priority 1: Critical Functionality Gaps**

### 1. **Dynamic Response Generation with Variables/Templates**

- **Current State**: Static responses only (hardcoded body, headers)
- **Enhancement**: Support template variables like `{{timestamp}}`, `{{uuid}}`, `{{request.path}}`, `{{request.headers.X-Custom}}`
- **Impact**: HIGH - Essential for realistic API mocking and testing dynamic scenarios
- **Files**: `crates/madhyamas-core/src/intercept/mock.rs:47-90`

### 2. **Response Sequencing/Scenarios**

- **Current State**: Same response every time
- **Enhancement**: Support response sequences (1st call → 200, 2nd call → 404, 3rd call → 500) or round-robin responses
- **Impact**: HIGH - Critical for testing retry logic, state transitions, and failure scenarios
- **Use Case**: Simulate flaky APIs, test circuit breakers

### 3. **Conditional Response Selection**

- **Current State**: Single response per rule
- **Enhancement**: Multiple responses with conditions (e.g., if `request.body.userId == "123"` → response A, else → response B)
- **Impact**: HIGH - Enables complex mocking scenarios without creating dozens of rules
- **Files**: `crates/madhyamas-core/src/intercept/mock.rs:47-62`

## **Priority 2: Developer Experience & Usability**

### 4. **Import Mock from HAR/OpenAPI/Postman**

- **Current State**: Manual creation only
- **Enhancement**: Import mocks from HAR files, OpenAPI specs, or Postman collections
- **Impact**: MEDIUM-HIGH - Massive time saver for developers
- **Files**: `crates/madhyamas-api/src/intercept_handlers.rs:142-172`

### 5. **Mock Recording from Live Traffic**

- **Current State**: Manual mock creation
- **Enhancement**: "Record" button to capture real responses and convert to mocks automatically
- **Impact**: MEDIUM-HIGH - Simplifies mock creation workflow
- **Integration Point**: `crates/madhyamas-core/src/proxy/engine.rs:318-322`

### 6. **Response Body File References (Enhanced)**

- **Current State**: Basic `body_file` support exists but no validation or UI
- **Enhancement**: File picker in UI, file watching/hot-reload, validation, relative path support
- **Impact**: MEDIUM - Better for large payloads
- **Files**: `crates/madhyamas-core/src/intercept/mock.rs:84-86`

### 7. **Mock Collections/Groups**

- **Current State**: Flat list of mocks
- **Enhancement**: Organize mocks into collections (e.g., "User API", "Payment API"), enable/disable entire groups
- **Impact**: MEDIUM - Better organization for large projects
- **Files**: `web/src/components/MocksPanel.tsx:84-458`

## **Priority 3: Advanced Matching & Control**

### 8. **Request Body Matching Enhancement**

- **Current State**: Basic regex body matching
- **Enhancement**: JSON path matching, XML XPath, GraphQL query matching, form data matching
- **Impact**: MEDIUM - More precise matching for complex APIs
- **Files**: `crates/madhyamas-core/src/intercept/types.rs:44-46`

### 9. **Probability-Based Responses**

- **Current State**: Deterministic responses
- **Enhancement**: Random responses with weights (70% success, 20% timeout, 10% error)
- **Impact**: MEDIUM - Chaos engineering and realistic testing
- **Use Case**: Test error handling under random failures

### 10. **Response Delay Variance/Jitter**

- **Current State**: Fixed delay only
- **Enhancement**: Delay ranges (e.g., 100-500ms), distribution patterns (normal, exponential)
- **Impact**: MEDIUM - More realistic latency simulation
- **Files**: `crates/madhyamas-core/src/proxy/engine.rs:1004-1006`

### 11. **Query Parameter Matching**

- **Current State**: URL pattern only (regex)
- **Enhancement**: Explicit query param matching (e.g., `?userId=123&status=active`)
- **Impact**: MEDIUM - Cleaner than complex regex patterns
- **Files**: `crates/madhyamas-core/src/intercept/types.rs:36-38`

## **Priority 4: Testing & Validation**

### 12. **Mock Response Validation**

- **Current State**: No validation
- **Enhancement**: Validate against JSON Schema, OpenAPI spec, or custom validators
- **Impact**: MEDIUM - Catch mock configuration errors early
- **Files**: `crates/madhyamas-api/src/intercept_handlers.rs:158-172`

### 13. **Mock Testing/Preview**

- **Current State**: No preview
- **Enhancement**: "Test Mock" button to see what response would be returned without affecting traffic
- **Impact**: MEDIUM - Faster debugging
- **Files**: `web/src/components/MocksPanel.tsx:126-170`

### 14. **Hit Count Analytics**

- **Current State**: Basic hit counter
- **Enhancement**: Hit history, timestamps, matched request details, analytics dashboard
- **Impact**: LOW-MEDIUM - Better visibility into mock usage
- **Files**: `crates/madhyamas-core/src/intercept/mock.rs:29`

## **Priority 5: Performance & Scalability**

### 15. **Response Caching**

- **Current State**: No caching
- **Enhancement**: Cache computed responses (especially for template rendering)
- **Impact**: LOW-MEDIUM - Performance optimization for high-traffic scenarios
- **Files**: `crates/madhyamas-core/src/proxy/engine.rs:999-1016`

### 16. **Lazy Loading for Large Response Bodies**

- **Current State**: Full body loaded into memory
- **Enhancement**: Stream large files instead of loading entirely
- **Impact**: LOW-MEDIUM - Better memory usage for large mocks
- **Files**: `crates/madhyamas-core/src/intercept/mock.rs:79-89`

## **Priority 6: Integration & Ecosystem**

### 17. **JavaScript/Lua Scripting for Response Generation**

- **Current State**: Static responses only
- **Enhancement**: Execute scripts to generate dynamic responses
- **Impact**: MEDIUM - Maximum flexibility
- **Note**: Script runtime already exists in codebase
- **Files**: `crates/madhyamas-api/src/lib.rs:32`

### 18. **Mock Sharing/Marketplace**

- **Current State**: Local only
- **Enhancement**: Share mock collections, community templates
- **Impact**: LOW - Community building
- **Files**: `crates/madhyamas-core/src/intercept/mock.rs:186-194`

### 19. **GraphQL Mock Support**

- **Current State**: Generic HTTP mocking
- **Enhancement**: GraphQL-specific matching (operation name, variables) and response generation
- **Impact**: LOW-MEDIUM - Better GraphQL developer experience
- **Files**: `crates/madhyamas-core/src/intercept/types.rs:31-54`

## **Priority 7: Quality of Life**

### 20. **Mock Duplication**

- **Current State**: No duplication feature
- **Enhancement**: "Duplicate" button to clone existing mocks
- **Impact**: LOW - Small UX improvement
- **Files**: `web/src/components/MocksPanel.tsx:180-186`

### 21. **Mock Versioning**

- **Current State**: No version history
- **Enhancement**: Track changes, rollback to previous versions
- **Impact**: LOW - Safety net for experimentation
- **Files**: `crates/madhyamas-core/src/persistence/intercept_store.rs:108-131`

### 22. **Expiration/TTL for Mocks**

- **Current State**: Mocks persist indefinitely
- **Enhancement**: Auto-disable/delete mocks after date/time or N hits
- **Impact**: LOW - Cleanup automation
- **Files**: `crates/madhyamas-core/src/intercept/mock.rs:12-30`

### 23. **Mock Comments/Documentation**

- **Current State**: Only name field
- **Enhancement**: Add description, tags, documentation fields
- **Impact**: LOW - Better team collaboration
- **Files**: `crates/madhyamas-core/src/intercept/mock.rs:12-30`

---

## Summary Statistics

- **Total Enhancements**: 23
- **High Priority**: 3 (Dynamic responses, Sequencing, Conditional selection)
- **Medium-High Priority**: 4 (Import, Recording, File references, Collections)
- **Medium Priority**: 8 (Advanced matching, validation, testing features)
- **Low-Medium Priority**: 4 (Performance, analytics)
- **Low Priority**: 4 (QoL improvements)

The top 3 enhancements would transform the mock system from basic to production-grade, enabling realistic API simulation and comprehensive testing scenarios.
