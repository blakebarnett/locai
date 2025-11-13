# Shell Completion Installation Guide

**Date**: 2025-11-27  
**Status**: Installation Instructions

## Overview

Locai CLI supports shell completion for bash, zsh, fish, PowerShell, and Elvish. This guide explains how to install and enable completions for each shell.

## Generating Completions

First, generate the completion script for your shell:

```bash
# Bash
locai-cli completions bash > locai-cli.bash

# Zsh
locai-cli completions zsh > _locai-cli

# Fish
locai-cli completions fish > locai-cli.fish

# PowerShell
locai-cli completions powershell > locai-cli.ps1

# Elvish
locai-cli completions elvish > locai-cli.elv
```

## Installation by Shell

### Bash

Bash does **not** automatically load completions from `~/.bash_completion.d/`. You have several options:

#### Option 1: System-wide Installation (Recommended for multi-user systems)

```bash
# Generate and install system-wide (requires sudo)
locai-cli completions bash | sudo tee /etc/bash_completion.d/locai-cli
```

This works if your system has `bash-completion` installed and configured (most Linux distributions do).

#### Option 2: User-specific Installation

**Method A: Direct sourcing in `.bashrc`**

```bash
# Generate completion script
locai-cli completions bash > ~/.locai-cli.bash

# Add to ~/.bashrc
echo 'source ~/.locai-cli.bash' >> ~/.bashrc

# Reload shell
source ~/.bashrc
```

**Method B: Using `~/.bash_completion.d/` (requires setup)**

If you want to use `~/.bash_completion.d/`, first add this to your `~/.bashrc`:

```bash
# Add to ~/.bashrc
if [ -d ~/.bash_completion.d ]; then
    for f in ~/.bash_completion.d/*.bash; do
        [ -f "$f" ] && source "$f"
    done
fi
```

Then install the completion:

```bash
# Create directory if it doesn't exist
mkdir -p ~/.bash_completion.d

# Generate and install
locai-cli completions bash > ~/.bash_completion.d/locai-cli.bash

# Reload shell
source ~/.bashrc
```

#### Option 3: Using `~/.local/share/bash-completion/completions/` (if bash-completion 2.0+)

```bash
# Create directory if it doesn't exist
mkdir -p ~/.local/share/bash-completion/completions

# Generate and install
locai-cli completions bash > ~/.local/share/bash-completion/completions/locai-cli

# Reload shell (bash-completion 2.0+ automatically loads from here)
source ~/.bashrc
```

### Zsh

#### Option 1: Using `$fpath` (Recommended)

```bash
# Find your zsh completion directory (usually one of these)
# Check which exists:
ls -d ~/.zsh/completions 2>/dev/null || \
ls -d ~/.oh-my-zsh/completions 2>/dev/null || \
ls -d /usr/local/share/zsh/site-functions 2>/dev/null

# Create directory if needed
mkdir -p ~/.zsh/completions

# Generate and install
locai-cli completions zsh > ~/.zsh/completions/_locai-cli

# Add to ~/.zshrc (if not already present)
echo 'fpath=(~/.zsh/completions $fpath)' >> ~/.zshrc
echo 'autoload -U compinit && compinit' >> ~/.zshrc

# Reload shell
source ~/.zshrc
```

#### Option 2: Direct sourcing

```bash
# Generate completion script
locai-cli completions zsh > ~/.locai-cli.zsh

# Add to ~/.zshrc
echo 'source ~/.locai-cli.zsh' >> ~/.zshrc

# Reload shell
source ~/.zshrc
```

### Fish

Fish automatically loads completions from `~/.config/fish/completions/`:

```bash
# Create directory if it doesn't exist
mkdir -p ~/.config/fish/completions

# Generate and install
locai-cli completions fish > ~/.config/fish/completions/locai-cli.fish

# Reload shell (or start new fish session)
source ~/.config/fish/config.fish
```

### PowerShell

#### Windows PowerShell

```powershell
# Generate completion script
locai-cli completions powershell > $PROFILE\locai-cli.ps1

# Or install to module directory
New-Item -ItemType Directory -Force -Path $env:USERPROFILE\Documents\PowerShell\Modules\locai-cli
locai-cli completions powershell > $env:USERPROFILE\Documents\PowerShell\Modules\locai-cli\locai-cli.ps1

# Add to profile
Add-Content $PROFILE "Import-Module $env:USERPROFILE\Documents\PowerShell\Modules\locai-cli\locai-cli.ps1"

# Reload profile
. $PROFILE
```

#### PowerShell Core (Cross-platform)

```powershell
# Generate completion script
locai-cli completions powershell > ~/.config/powershell/locai-cli.ps1

# Add to profile
Add-Content $PROFILE "source ~/.config/powershell/locai-cli.ps1"

# Reload profile
. $PROFILE
```

### Elvish

Elvish automatically loads completions from `~/.config/elvish/lib/`:

```bash
# Create directory if it doesn't exist
mkdir -p ~/.config/elvish/lib

# Generate and install
locai-cli completions elvish > ~/.config/elvish/lib/locai-cli.elv

# Reload shell (or start new elvish session)
```

## Verification

After installation, verify completions work:

```bash
# Start a new shell session, then try:
locai-cli <TAB>          # Should show available commands
locai-cli memory <TAB>  # Should show memory subcommands
```

## Troubleshooting

### Bash: Completions not working

1. **Check if bash-completion is installed**:
   ```bash
   which bash_completion || echo "bash-completion not found"
   ```

2. **Check if bash-completion is sourced**:
   ```bash
   grep -q "bash_completion" ~/.bashrc && echo "Found" || echo "Not found"
   ```

3. **Try direct sourcing** (see Option 2A above)

4. **Check file permissions**:
   ```bash
   ls -l ~/.locai-cli.bash  # Should be readable
   ```

### Zsh: Completions not working

1. **Check if compinit is called**:
   ```bash
   grep -q "compinit" ~/.zshrc && echo "Found" || echo "Not found"
   ```

2. **Ensure fpath includes your completion directory**:
   ```bash
   echo $fpath | grep -q ".zsh/completions" && echo "Found" || echo "Not found"
   ```

3. **Try direct sourcing** (see Option 2 above)

### Fish: Completions not working

1. **Check completion file location**:
   ```bash
   ls -l ~/.config/fish/completions/locai-cli.fish
   ```

2. **Check file syntax**:
   ```bash
   fish -n ~/.config/fish/completions/locai-cli.fish
   ```

## Quick Install Script

For convenience, here's a quick install script for bash:

```bash
#!/bin/bash
# Quick install script for bash completions

COMPLETION_FILE="$HOME/.locai-cli.bash"

# Generate completion
locai-cli completions bash > "$COMPLETION_FILE"

# Add to .bashrc if not already present
if ! grep -q "locai-cli.bash" ~/.bashrc 2>/dev/null; then
    echo "" >> ~/.bashrc
    echo "# Locai CLI completions" >> ~/.bashrc
    echo "source $COMPLETION_FILE" >> ~/.bashrc
    echo "✓ Added to ~/.bashrc"
else
    echo "✓ Already in ~/.bashrc"
fi

echo "✓ Completion installed. Run 'source ~/.bashrc' to activate."
```

Save as `install-completions.sh`, make executable (`chmod +x install-completions.sh`), and run it.

## Notes

- **Bash**: Requires explicit sourcing or system bash-completion setup
- **Zsh**: Requires `compinit` and proper `fpath` configuration
- **Fish**: Automatically loads from `~/.config/fish/completions/`
- **PowerShell**: Requires profile configuration
- **Elvish**: Automatically loads from `~/.config/elvish/lib/`

The `completions` command generates completion scripts but does not install them automatically. This gives you control over where and how completions are installed.


