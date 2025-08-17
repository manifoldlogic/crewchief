# CrewChief Web UI Dockerfile
# Multi-stage build for optimized production image

# ========================================
# Build stage - Node.js application build
# ========================================
FROM node:20-alpine AS builder

# Install build dependencies
RUN apk add --no-cache \
    python3 \
    make \
    g++ \
    libc6-compat

# Enable pnpm
RUN corepack enable pnpm

# Set working directory
WORKDIR /app

# Copy package.json and workspace files
COPY package.json pnpm-lock.yaml pnpm-workspace.yaml ./
COPY packages/web-ui/package.json ./packages/web-ui/

# Install dependencies with frozen lockfile
RUN pnpm install --frozen-lockfile

# Copy source code
COPY packages/web-ui/ ./packages/web-ui/

# Set working directory to web-ui package
WORKDIR /app/packages/web-ui

# Build the application
RUN pnpm build

# ========================================
# Production stage - Minimal runtime image
# ========================================
FROM node:20-alpine AS production

# Install runtime dependencies and security updates
RUN apk add --no-cache \
    dumb-init \
    curl \
    ca-certificates \
    && apk upgrade --no-cache

# Create non-root user for security
RUN addgroup -g 1001 -S nodejs \
    && adduser -S crewchief -u 1001 -G nodejs

# Set working directory
WORKDIR /app

# Enable pnpm for production dependencies
RUN corepack enable pnpm

# Copy package files for production install
COPY --chown=crewchief:nodejs package.json pnpm-lock.yaml pnpm-workspace.yaml ./
COPY --chown=crewchief:nodejs packages/web-ui/package.json ./packages/web-ui/

# Install only production dependencies
RUN pnpm install --frozen-lockfile --prod \
    && pnpm store prune \
    && rm -rf /root/.cache /tmp/*

# Copy built application from builder stage
COPY --from=builder --chown=crewchief:nodejs /app/packages/web-ui/dist ./packages/web-ui/dist

# Copy any required static files
COPY --from=builder --chown=crewchief:nodejs /app/packages/web-ui/public ./packages/web-ui/public

# Set working directory to web-ui package
WORKDIR /app/packages/web-ui

# Expose application port
EXPOSE 3456

# Set environment variables
ENV NODE_ENV=production
ENV PORT=3456

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=30s --retries=3 \
    CMD curl -f http://localhost:3456/api/health || exit 1

# Switch to non-root user
USER crewchief

# Use dumb-init to handle signals properly
ENTRYPOINT ["dumb-init", "--"]

# Start the application
CMD ["pnpm", "start"]