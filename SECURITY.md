# Security Policy

## 🔒 Security Overview

Simple Docker Manager is a powerful tool that provides web-based access to Docker containers. Due to the sensitive nature of container management, we take security seriously and have implemented multiple layers of protection.

## 🚨 Security Considerations

**⚠️ CRITICAL WARNING**: This tool provides web-based control over Docker containers. Improper deployment can lead to:

- Unauthorized container access
- Container manipulation by attackers
- Privilege escalation attacks
- Data exposure from managed containers
- Host system compromise

**Only deploy this tool in trusted environments with proper security controls.**

## 🛡️ Built-in Security Features

### Authentication & Authorization

- **Mandatory Authentication**: Authentication is enabled by default
- **Secure Password Handling**: Argon2 password hashing with secure defaults
- **Session Management**: Configurable session timeouts with secure cookies
- **Auto-generated Passwords**: Cryptographically secure password generation when not provided

### Container Security

- **Minimal Attack Surface**: Built on `scratch` base image with no shell or package manager
- **Non-root Execution**: Application runs as non-privileged user (UID 10001)
- **Read-only Docker Socket**: Docker socket mounted read-only by default
- **Static Binary**: Fully statically linked with no runtime dependencies

### Network Security

- **HTTPS Ready**: Designed to run behind reverse proxy with TLS termination
- **Configurable Binding**: Can bind to specific interfaces (not just 0.0.0.0)
- **Health Endpoints**: Separate health/readiness endpoints for monitoring

## 🔍 Security Scanning

This project undergoes automated security scanning:

- **Trivy**: Container vulnerability scanning
- **Cargo Audit**: Rust dependency vulnerability scanning
- **Cargo Deny**: License and dependency policy enforcement
- **Semgrep**: Static Application Security Testing (SAST)
- **GitLeaks**: Secret detection in code and history
- **Hadolint**: Dockerfile security linting

All scans run on every commit and weekly via GitHub Actions.

## 📋 Supported Versions

We provide security updates for the following versions:

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |
| < 0.1   | :x:                |

## 🚨 Reporting a Vulnerability

**DO NOT** report security vulnerabilities through public GitHub issues.

### Preferred Method: GitHub Security Advisories

1. Go to the [Security tab](https://github.com/OscillateLabsLLC/simple-docker-manager/security) of this repository
2. Click "Report a vulnerability"
3. Fill out the security advisory form with:
   - Detailed description of the vulnerability
   - Steps to reproduce
   - Potential impact assessment
   - Suggested mitigation (if known)

### Alternative: Email

If you cannot use GitHub Security Advisories, email: **security@oscillatelabs.com**

**Include in your report:**

- Description of the vulnerability
- Steps to reproduce the issue
- Potential impact and attack scenarios
- Your assessment of severity
- Any suggested fixes or mitigations

### What to Expect

- **Acknowledgment**: Within 1 week
- **Initial Assessment**: Within 2 weeks
- **Resolution Timeline**: Best effort based on severity and maintainer availability

### Disclosure Policy

- We follow **coordinated disclosure**
- We will work with you to understand and fix the issue
- We will not take legal action against researchers who:
  - Follow this policy
  - Act in good faith
  - Do not access data beyond what's necessary to demonstrate the vulnerability
  - Do not intentionally harm our users or systems

## 🏆 Security Recognition

We believe in recognizing security researchers who help improve our security:

- **Public Recognition**: With your permission, we'll acknowledge your contribution
- **CVE Assignment**: For qualifying vulnerabilities
- **Security Advisory**: Detailed public disclosure after fix is deployed

## 🔧 Security Best Practices for Deployment

### Production Deployment

1. **Use HTTPS**: Always deploy behind a reverse proxy with TLS

   ```nginx
   server {
       listen 443 ssl;
       ssl_certificate /path/to/cert.pem;
       ssl_certificate_key /path/to/key.pem;

       location / {
           proxy_pass http://localhost:3000;
           proxy_set_header Host $host;
           proxy_set_header X-Real-IP $remote_addr;
       }
   }
   ```

2. **Network Isolation**: Use Docker networks or firewall rules

   ```bash
   # Create isolated network
   docker network create --driver bridge sdm-network

   # Run with network isolation
   docker run --network sdm-network simple-docker-manager
   ```

3. **Strong Authentication**: Use strong passwords and short session timeouts

   ```bash
   SDM_AUTH_PASSWORD="$(openssl rand -base64 32)"
   SDM_SESSION_TIMEOUT_SECONDS=1800  # 30 minutes
   ```

4. **Resource Limits**: Set container resource constraints

   ```yaml
   services:
     simple-docker-manager:
       deploy:
         resources:
           limits:
             memory: 256M
             cpus: "0.5"
   ```

5. **Read-only Filesystem**: Mount application directories read-only
   ```yaml
   services:
     simple-docker-manager:
       read_only: true
       tmpfs:
         - /tmp
   ```

### Monitoring & Alerting

1. **Log Monitoring**: Monitor authentication failures and unusual activity
2. **Health Checks**: Implement proper health monitoring
3. **Access Logs**: Log all access attempts with source IPs
4. **Container Monitoring**: Monitor managed containers for suspicious activity

### Access Control

1. **Principle of Least Privilege**: Only grant necessary Docker permissions
2. **Network Segmentation**: Isolate from production networks
3. **VPN/Bastion Access**: Require VPN or bastion host access
4. **IP Allowlisting**: Restrict access to known IP ranges

## 🚫 Security Anti-patterns

**DO NOT:**

- Expose directly to the internet without authentication
- Use weak or default passwords
- Run with `--privileged` flag
- Mount Docker socket as read-write unless absolutely necessary
- Disable authentication in production
- Use HTTP in production environments
- Run as root user
- Expose on 0.0.0.0 in untrusted networks

## 📚 Security Resources

- [Docker Security Best Practices](https://docs.docker.com/engine/security/)
- [OWASP Container Security](https://owasp.org/www-project-container-security/)
- [CIS Docker Benchmark](https://www.cisecurity.org/benchmark/docker)

## 📞 Contact

- **Security Issues**: security@oscillatelabs.com
- **General Questions**: Use GitHub Discussions
- **Bug Reports**: Use GitHub Issues (for non-security bugs only)

---

**Remember**: Security is a shared responsibility. While we work hard to make this tool secure, proper deployment and operational security practices are essential for maintaining security in your environment.
