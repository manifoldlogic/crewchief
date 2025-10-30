#!/bin/bash
set -e

echo "=== v1.1.10 Rollback Script ==="
echo ""
echo "This script helps roll back v1.1.10 to v1.1.9"
echo ""

# Verify current published version
echo "Step 1: Checking current npm version..."
CURRENT_VERSION=$(npm view @crewchief/maproom-mcp version)
echo "Current version: $CURRENT_VERSION"

if [ "$CURRENT_VERSION" != "1.1.10" ]; then
  echo "❌ Current version is not 1.1.10, rollback may not be needed"
  read -p "Continue anyway? (y/N) " -n 1 -r
  echo
  if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    exit 0
  fi
fi

# Check 72-hour window
echo ""
echo "Step 2: Checking npm publish time..."
npm view @crewchief/maproom-mcp time --json

echo ""
read -p "Is v1.1.10 within 72 hours of publish? (y/N) " -n 1 -r
echo
WITHIN_72HR=$REPLY

# Unpublish if possible
if [[ $WITHIN_72HR =~ ^[Yy]$ ]]; then
  echo ""
  echo "Step 3: Unpublishing v1.1.10 from npm..."
  read -p "Proceed with npm unpublish? (y/N) " -n 1 -r
  echo
  if [[ $REPLY =~ ^[Yy]$ ]]; then
    npm unpublish @crewchief/maproom-mcp@1.1.10
    echo "✅ Unpublished v1.1.10"
  fi
else
  echo ""
  echo "⚠️  Beyond 72-hour window. npm unpublish not available."
  echo "You must publish v1.1.11 to supersede v1.1.10"
  read -p "Create v1.1.11 revert? (y/N) " -n 1 -r
  echo
  if [[ $REPLY =~ ^[Yy]$ ]]; then
    echo "Manual steps required:"
    echo "1. Revert docker-compose.yml changes"
    echo "2. Update package.json version to 1.1.11"
    echo "3. Run: npm publish --access public"
  fi
fi

# Docker Hub cleanup
echo ""
echo "Step 4: Docker Hub cleanup..."
echo "Manual steps required:"
echo "1. Visit: https://hub.docker.com/r/crewchief/maproom-mcp/tags"
echo "2. Delete tags: 1.1.10, 1.1.10-rc1, 1.1.10-rc2"
echo "3. Verify latest tag points to 1.1.9"

# Git tag handling
echo ""
echo "Step 5: Git tag handling..."
read -p "Delete git tag v1.1.10? (y/N) " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
  git tag -d v1.1.10
  git push origin :refs/tags/v1.1.10
  echo "✅ Deleted git tag v1.1.10"
fi

# Communication
echo ""
echo "Step 6: User communication..."
echo "Template for GitHub Release / README:"
echo ""
cat << 'TEMPLATE'
# ⚠️ v1.1.10 Rollback Notice

**v1.1.10 has been rolled back due to critical issues.**

## Action Required

If you installed v1.1.10:
1. Uninstall: `npm uninstall -g @crewchief/maproom-mcp`
2. Reinstall v1.1.9: `npm install -g @crewchief/maproom-mcp@1.1.9`
3. Restart services: `maproom-mcp restart`

## What Happened

[Describe the issue]

## Next Steps

We are working on v1.1.11 with fixes. Expected release: [DATE]

## Questions

Please file issues at: https://github.com/danielbushman/crewchief/issues
TEMPLATE

echo ""
echo "✅ Rollback checklist complete!"
echo ""
echo "Don't forget to:"
echo "- Update README.md with warning"
echo "- Post GitHub Release"
echo "- Monitor for user reports"
