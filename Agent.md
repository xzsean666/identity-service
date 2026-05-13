# Agent.md

## Purpose

This file defines how AI agents must work in this repository.

Project-specific architecture, specifications, build notes, and handoff records live in `docs/`.

## Project Identity

This repository is the Identity Platform / IAM Service.

The service provides unified identity, authentication, and authorization capabilities for:

- Web applications
- Mobile applications
- OpenAPI consumers
- Third-party applications
- Internal microservices
- API gateways

The platform aggregates multiple identity providers and maps all external identities to one internal identity model based on `internal_user_id`.

## Required Execution Workflow

AI agents must follow the workflow below in order.

### Step 1 - Architecture Design

Must be completed first.

Output must include:

- Overall system architecture
- Module breakdown with clear responsibilities
- Data flow between modules
- Key design decisions

Do not write implementation code in this step.

Primary document:

- `docs/ARCHITECTURE.md`

### Step 2 - Documentation

Generate and maintain:

- `docs/SPEC.md`
- `docs/BUILD.md`

Do not write implementation code in this step.

### Step 3 - Context Handoff

Generate and maintain:

- `docs/nextsession.md`

The handoff document must include:

- Current progress
- Architecture summary
- Completed parts
- Pending tasks
- Next actions
- Risks and unknowns

### Step 4 - Implementation

Only start implementation after explicit user approval.

Implementation must follow the architecture and specification documents.

## Engineering Principles

### Optimize for AI Comprehension

The codebase must be easy for an AI agent to understand, modify, and extend within limited context.

Prefer explicit, local, predictable modules over clever or highly abstract designs.

### Cognitive-Based Decomposition

Split modules by whether each module can be understood in isolation.

Do not split only by line count, file size, or framework convention.

### Single Responsibility

Each module must have:

- One clear purpose
- Explicit inputs
- Explicit outputs
- Minimal side effects
- Clearly imported dependencies

### Naming as Documentation

Names must describe intent clearly.

Avoid unclear abbreviations such as:

- `cfg`
- `tmp`
- `svc`
- `mgr`

Use names such as:

- `authentication_request`
- `identity_provider_adapter`
- `session_lifecycle_policy`
- `authorization_decision`

### Explicit Behavior

Avoid hidden state, implicit injections, magic configuration, and cross-file behavior that is hard to trace.

Configuration must be centralized when introduced.

### Composition over Inheritance

Prefer small composed modules and interfaces over inheritance hierarchies.

Inheritance may only be used when it is simple, shallow, and justified by the architecture.

### Incremental Buildability

The system must be buildable and testable step by step.

Each feature should be introduced through a small, reviewable increment.

## Documentation Layout

All project documents except this file must live in `docs/`.

Required documents:

- `docs/ARCHITECTURE.md` - architecture design
- `docs/SPEC.md` - system specification
- `docs/BUILD.md` - build and usage instructions
- `docs/nextsession.md` - context handoff for future sessions

## Git Workflow

After each major step, run:

```bash
git add .
git commit -m "feat: <describe current step>"
```

Do not push unless the user explicitly asks.

## Self-Correction Rule

If an AI agent detects premature coding, poor modularization, hidden behavior, or increasing complexity, it must stop and adjust the architecture or documentation before continuing.

## Current Implementation Boundary

No production implementation exists yet.

The current approved scope is documentation and architecture preparation only.
