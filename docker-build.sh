#!/bin/bash

# Simple Docker Manager - Docker Build Script
# This script builds and optionally runs the Docker container

set -e

# Configuration
IMAGE_NAME="simple-docker-manager"
TAG="latest"
REGISTRY="" # Set this if you want to push to a registry

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Function to show usage
show_usage() {
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "Options:"
    echo "  -t, --tag TAG          Specify the image tag (default: latest)"
    echo "  -r, --registry URL     Specify registry URL for tagging"
    echo "  -p, --push             Push image to registry after build"
    echo "  --run                  Run the container after building"
    echo "  --no-cache             Build without using cache"
    echo "  --platform PLATFORM    Build for specific platform (e.g., linux/amd64,linux/arm64)"
    echo "  --multi-arch           Build for multiple architectures (linux/amd64,linux/arm64)"
    echo "  -h, --help             Show this help message"
    echo ""
    echo "Examples:"
    echo "  $0                                    # Build for current platform"
    echo "  $0 --tag v1.0.0 --run               # Build v1.0.0 and run"
    echo "  $0 --registry ghcr.io/user --push   # Build and push to GitHub Container Registry"
    echo "  $0 --platform linux/amd64           # Build specifically for x86_64"
    echo "  $0 --multi-arch --push              # Build for multiple architectures and push"
    echo "  $0 --no-cache                       # Build without cache"
}

# Parse command line arguments
PUSH=false
RUN_CONTAINER=false
NO_CACHE=""
PLATFORM=""
MULTI_ARCH=false

while [[ $# -gt 0 ]]; do
    case $1 in
        -t|--tag)
            TAG="$2"
            shift 2
            ;;
        -r|--registry)
            REGISTRY="$2"
            shift 2
            ;;
        -p|--push)
            PUSH=true
            shift
            ;;
        --run)
            RUN_CONTAINER=true
            shift
            ;;
        --no-cache)
            NO_CACHE="--no-cache"
            shift
            ;;
        --platform)
            PLATFORM="$2"
            shift 2
            ;;
        --multi-arch)
            MULTI_ARCH=true
            PLATFORM="linux/amd64,linux/arm64"
            shift
            ;;
        -h|--help)
            show_usage
            exit 0
            ;;
        *)
            print_error "Unknown option: $1"
            show_usage
            exit 1
            ;;
    esac
done

# Construct full image name
if [[ -n "$REGISTRY" ]]; then
    FULL_IMAGE_NAME="${REGISTRY}/${IMAGE_NAME}:${TAG}"
else
    FULL_IMAGE_NAME="${IMAGE_NAME}:${TAG}"
fi

print_status "Building Docker image: $FULL_IMAGE_NAME"

# Check if Docker is running
if ! docker info >/dev/null 2>&1; then
    print_error "Docker is not running or not accessible"
    exit 1
fi

# Setup buildx if using multi-platform builds
if [[ -n "$PLATFORM" ]]; then
    print_status "Setting up Docker buildx for multi-platform build..."
    
    # Check if buildx is available
    if ! docker buildx version >/dev/null 2>&1; then
        print_error "Docker buildx is not available. Please upgrade to a newer version of Docker."
        exit 1
    fi
    
    # Create or use existing builder
    BUILDER_NAME="simple-docker-manager-builder"
    if ! docker buildx inspect "$BUILDER_NAME" >/dev/null 2>&1; then
        print_status "Creating new buildx builder: $BUILDER_NAME"
        docker buildx create --name "$BUILDER_NAME" --use
    else
        print_status "Using existing buildx builder: $BUILDER_NAME"
        docker buildx use "$BUILDER_NAME"
    fi
    
    # Bootstrap the builder
    docker buildx inspect --bootstrap
fi

# Prepare build arguments
BUILD_ARGS=""
BUILD_ARGS="$BUILD_ARGS --build-arg VERSION=$TAG"
BUILD_ARGS="$BUILD_ARGS --build-arg BUILD_DATE=$(date -u +'%Y-%m-%dT%H:%M:%SZ')"

# Add git commit if available
if git rev-parse --short HEAD >/dev/null 2>&1; then
    BUILD_ARGS="$BUILD_ARGS --build-arg VCS_REF=$(git rev-parse --short HEAD)"
fi

# Build the image
print_status "Starting Docker build..."

if [[ -n "$PLATFORM" ]]; then
    # Multi-platform build using buildx
    BUILD_CMD="docker buildx build"
    BUILD_CMD="$BUILD_CMD --platform $PLATFORM"
    BUILD_CMD="$BUILD_CMD $NO_CACHE"
    BUILD_CMD="$BUILD_CMD $BUILD_ARGS"
    BUILD_CMD="$BUILD_CMD -t $FULL_IMAGE_NAME"
    
    if [[ "$PUSH" == true ]]; then
        BUILD_CMD="$BUILD_CMD --push"
    else
        # For multi-arch without push, we need to specify output
        if [[ "$MULTI_ARCH" == true ]]; then
            BUILD_CMD="$BUILD_CMD --output type=registry"
            print_warning "Multi-arch builds require pushing to registry. Use --push flag."
        else
            BUILD_CMD="$BUILD_CMD --load"
        fi
    fi
    
    BUILD_CMD="$BUILD_CMD ."
    
    print_status "Build command: $BUILD_CMD"
    
    if eval $BUILD_CMD; then
        print_success "Successfully built $FULL_IMAGE_NAME"
    else
        print_error "Failed to build Docker image"
        exit 1
    fi
else
    # Regular single-platform build
    if docker build $NO_CACHE $BUILD_ARGS -t "$FULL_IMAGE_NAME" .; then
        print_success "Successfully built $FULL_IMAGE_NAME"
    else
        print_error "Failed to build Docker image"
        exit 1
    fi
    
    # Show image size for single-platform builds
    IMAGE_SIZE=$(docker images "$FULL_IMAGE_NAME" --format "table {{.Repository}}:{{.Tag}}\t{{.Size}}" | tail -n 1 | awk '{print $2}')
    print_status "Image size: $IMAGE_SIZE"
    
    # Push to registry if requested (single-platform)
    if [[ "$PUSH" == true ]]; then
        if [[ -z "$REGISTRY" ]]; then
            print_error "Cannot push: no registry specified. Use --registry option."
            exit 1
        fi
        
        print_status "Pushing image to registry..."
        if docker push "$FULL_IMAGE_NAME"; then
            print_success "Successfully pushed $FULL_IMAGE_NAME"
        else
            print_error "Failed to push Docker image"
            exit 1
        fi
    fi
fi

# Run the container if requested (only for single-platform builds)
if [[ "$RUN_CONTAINER" == true ]]; then
    if [[ -n "$PLATFORM" && "$PLATFORM" == *","* ]]; then
        print_warning "Cannot run container after multi-platform build. Skipping run step."
    else
        print_status "Starting container..."
        
        # Stop and remove existing container if it exists
        if docker ps -a --format '{{.Names}}' | grep -q "^${IMAGE_NAME}$"; then
            print_warning "Stopping and removing existing container..."
            docker stop "$IMAGE_NAME" >/dev/null 2>&1 || true
            docker rm "$IMAGE_NAME" >/dev/null 2>&1 || true
        fi
        
        # Run the new container
        docker run -d \
            --name "$IMAGE_NAME" \
            -p 3000:3000 \
            -v /var/run/docker.sock:/var/run/docker.sock:ro \
            --restart unless-stopped \
            "$FULL_IMAGE_NAME"
        
        print_success "Container started successfully!"
        print_status "Access the application at: http://localhost:3000"
        print_status "View logs with: docker logs -f $IMAGE_NAME"
        print_status "Stop container with: docker stop $IMAGE_NAME"
    fi
fi

print_success "Build process completed!"

# Show useful commands
echo ""
echo "Useful commands:"
echo "  docker run -d --name $IMAGE_NAME -p 3000:3000 -v /var/run/docker.sock:/var/run/docker.sock:ro $FULL_IMAGE_NAME"
echo "  docker logs -f $IMAGE_NAME"
echo "  docker exec -it $IMAGE_NAME sh  # Won't work with scratch - use for debugging only"
echo "  docker stop $IMAGE_NAME"
echo "  docker rm $IMAGE_NAME"

# Show platform-specific examples
if [[ -n "$PLATFORM" ]]; then
    echo ""
    echo "Platform-specific examples:"
    echo "  ./docker-build.sh --platform linux/amd64    # Build for x86_64"
    echo "  ./docker-build.sh --platform linux/arm64    # Build for ARM64"
    echo "  ./docker-build.sh --multi-arch --push       # Build for both architectures"
fi 