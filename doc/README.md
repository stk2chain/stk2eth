# Documentation - Project Specifications & Design

## Overview

The `doc` directory contains comprehensive project documentation, including system specifications, architecture diagrams, contributing guidelines, and technical design documents for the STK2ETH project.

## Documentation Structure

This directory serves as the central hub for all project documentation:
- **System Architecture** - High-level system design and component interactions
- **Technical Specifications** - Detailed technical requirements and specifications
- **Visual Diagrams** - System schematics and architectural illustrations
- **Contributing Guidelines** - Development workflow and contribution standards

## Documents

### SPEC.md
Core technical specification document for the STK2ETH project.

**Contents:**
- System requirements and constraints
- Component specifications
- API definitions and interfaces
- Data models and schemas
- Security requirements
- Performance criteria

**Target Audience:** Developers, architects, and technical stakeholders

### System_Schematics-2025-06-30-2208.excalidraw.png
Comprehensive system architecture diagram created with Excalidraw.

**Visual Elements:**
- Component interactions and data flows
- Network topology and communication paths
- User journey and interaction patterns
- Technology stack visualization
- Deployment architecture

**Usage:** Reference for understanding system-wide interactions and dependencies

### Consumer_eUICC.excalidraw.svg
Detailed diagram of consumer eUICC (eSIM) integration.

**Focus Areas:**
- eSIM profile management
- Device-to-network communication
- Account abstraction wallet binding
- Offline transaction capabilities

**Format:** SVG for scalable viewing and embedding in documentation

### Mobile_ETH.png
Mobile Ethereum interaction flow diagram.

**Illustrates:**
- Mobile device Ethereum operations
- USSD-based transaction flows
- Account abstraction implementation
- User experience design

## Contributing Guidelines

### contributing/
Subdirectory containing detailed contribution guidelines and development processes.

**Structure:**
```
contributing/
├── CODE_OF_CONDUCT.md      # Community guidelines
├── DEVELOPMENT_SETUP.md    # Development environment setup
├── CODING_STANDARDS.md     # Code style and conventions
├── TESTING_GUIDELINES.md   # Testing requirements and best practices
├── REVIEW_PROCESS.md       # Code review workflow
└── RELEASE_PROCESS.md      # Release management procedures
```

## Specifications

### specs/
Subdirectory containing detailed technical specifications.

**Structure:**
```
specs/
├── USSD_PROTOCOL.md        # USSD interaction specifications
├── SPACETIMEDB_SCHEMA.md   # Database schema definitions
├── ETHEREUM_INTEGRATION.md # Blockchain integration specs
├── ACCOUNT_ABSTRACTION.md  # AA wallet specifications
├── FATF_COMPLIANCE.md      # Regulatory compliance requirements
└── API_REFERENCE.md        # Complete API documentation
```

## Usage Guidelines

### For Developers
1. **Start with SPEC.md** - Understand overall system requirements
2. **Review system diagrams** - Visualize component interactions
3. **Check contributing guidelines** - Follow development standards
4. **Reference API docs** - Use correct interfaces and data formats

### For Architects
1. **System schematics** - Understand deployment architecture
2. **Component specifications** - Design integration points
3. **Performance requirements** - Plan capacity and scaling
4. **Security specifications** - Implement proper security measures

### For Product Managers
1. **User flow diagrams** - Understand user experience
2. **Technical constraints** - Plan feature development
3. **Compliance requirements** - Ensure regulatory adherence
4. **System capabilities** - Define product roadmap

## Document Management

### Version Control
All documentation is version-controlled alongside code:
- Use meaningful commit messages for documentation changes
- Tag documentation with release versions
- Maintain change logs for major specification updates

### Review Process
Documentation changes follow the same review process as code:
1. Create feature branch for documentation updates
2. Submit pull request with clear description
3. Obtain review from technical team
4. Merge after approval

### Format Standards

#### Markdown Documents
- Use consistent heading hierarchy
- Include table of contents for long documents
- Follow common markdown conventions
- Add code syntax highlighting where appropriate

#### Diagrams
- Use vector formats (SVG) when possible
- Include source files for editable diagrams (Excalidraw, etc.)
- Maintain consistent visual styling
- Include alt text for accessibility

#### Images
- Optimize file sizes for web viewing
- Use descriptive filenames
- Include captions and context
- Store in appropriate subdirectories

## Diagram Sources

### Excalidraw Files
Original diagram source files for editing:
- `Consumer_eUICC.excalidraw` (source for .svg)
- `System_Schematics-2025-06-30-2208.excalidraw` (source for .png)

**Editing:** Import these files into [Excalidraw](https://excalidraw.com) for modifications

### Image Assets
Static image files for documentation:
- PNG files for screenshots and complex diagrams
- SVG files for scalable vector graphics
- High-resolution versions for print documentation

## Documentation Workflow

### Creating New Documentation
1. **Identify need** - Gap analysis or new feature requirements
2. **Choose format** - Markdown, diagram, or specification
3. **Create draft** - Follow existing templates and standards
4. **Internal review** - Team review for accuracy and completeness
5. **Publication** - Merge to main branch and update indexes

### Updating Existing Documentation
1. **Identify changes** - Code changes requiring documentation updates
2. **Update affected docs** - Maintain consistency across all documents
3. **Version appropriately** - Track significant changes
4. **Cross-reference** - Update related documents and diagrams

### Documentation Maintenance
- **Regular reviews** - Quarterly documentation audits
- **Link validation** - Check for broken internal/external links
- **Accuracy verification** - Ensure docs match current implementation
- **Accessibility** - Maintain screen reader compatibility

## Integration with Development

### Code Documentation
- API documentation generated from code comments
- Architecture Decision Records (ADRs) for significant choices
- Inline code comments referencing specifications

### Testing Documentation
- Test plan documentation
- Coverage reports and analysis
- Performance benchmarking results

### Deployment Documentation
- Infrastructure setup guides
- Configuration management docs
- Monitoring and alerting specifications

## External Resources

### Related Documentation
- [SpacetimeDB Documentation](https://spacetimedb.com/docs)
- [Foundry Book](https://book.getfoundry.sh/)
- [FATF Guidance](https://www.fatf-gafi.org/publications/virtualassets/)
- [Account Abstraction EIPs](https://eips.ethereum.org/EIPS/eip-4337)

### Tools and Standards
- [Excalidraw](https://excalidraw.com) - Diagram creation
- [Mermaid](https://mermaid-js.github.io/) - Code-generated diagrams
- [RFC 2119](https://tools.ietf.org/html/rfc2119) - Requirement specification keywords
- [CommonMark](https://commonmark.org/) - Markdown specification

## Related Components

This documentation supports all project components:
- **ussdgeth** - SpacetimeDB module specifications
- **ussdclient** - HTTP bridge API documentation
- **ethclient** - Ethereum integration specifications
- **contracts** - Smart contract interface docs
- **tests** - Testing strategy and requirements