<script lang="ts">
  import { isAuthenticated, user } from "$lib/stores/auth";
  import { goto } from "$app/navigation";
  import { onMount } from "svelte";

  onMount(() => {
    // Redirect based on auth state
    if ($isAuthenticated) {
      goto("/dashboard");
    }
  });
</script>

<div class="landing">
  <div class="hero">
    <div class="hero-content">
      <h1 class="hero-title">
        <span class="gradient-text">SaaS Boilerplate</span>
      </h1>
      <p class="hero-subtitle">Built with Rust + Tauri + Svelte</p>
      <p class="hero-description">
        A modern, secure, and performant foundation for your next application.
        Authentication, user management, and settings - all ready to go.
      </p>

      <div class="hero-actions">
        <a href="/login" class="btn btn-primary btn-lg"> Get Started </a>
        <a href="/register" class="btn btn-secondary btn-lg">
          Create Account
        </a>
      </div>

      <div class="features">
        <div class="feature">
          <div class="feature-icon">🔐</div>
          <div class="feature-text">
            <strong>Secure Auth</strong>
            <span>JWT + Argon2</span>
          </div>
        </div>
        <div class="feature">
          <div class="feature-icon">⚡</div>
          <div class="feature-text">
            <strong>Blazing Fast</strong>
            <span>Rust Backend</span>
          </div>
        </div>
        <div class="feature">
          <div class="feature-icon">🎨</div>
          <div class="feature-text">
            <strong>Modern UI</strong>
            <span>Svelte + CSS</span>
          </div>
        </div>
      </div>
    </div>

    <div class="hero-visual">
      <div class="glow-orb orb-1"></div>
      <div class="glow-orb orb-2"></div>
      <div class="code-block">
        <pre><code
            ><span class="keyword">const</span> app = <span class="keyword"
              >await</span
            > <span class="function">invoke</span>(<span class="string"
              >'login'</span
            >, &#123;
    email: <span class="string">'user@example.com'</span>,
    password: <span class="string">'secure123'</span>
&#125;);</code
          ></pre>
      </div>
    </div>
  </div>
</div>

<style>
  .landing {
    min-height: 100vh;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 2rem;
    position: relative;
    overflow: hidden;
  }

  .hero {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 4rem;
    max-width: 1200px;
    width: 100%;
  }

  .hero-content {
    display: flex;
    flex-direction: column;
    justify-content: center;
  }

  .hero-title {
    font-size: 3.5rem;
    font-weight: 700;
    margin-bottom: 1rem;
  }

  .gradient-text {
    background: linear-gradient(
      135deg,
      var(--color-primary),
      var(--color-secondary)
    );
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
  }

  .hero-subtitle {
    font-size: 1.25rem;
    color: var(--text-secondary);
    margin-bottom: 1rem;
  }

  .hero-description {
    color: var(--text-muted);
    font-size: 1rem;
    max-width: 500px;
    margin-bottom: 2rem;
  }

  .hero-actions {
    display: flex;
    gap: 1rem;
    margin-bottom: 3rem;
  }

  .btn-lg {
    padding: 1rem 2rem;
    font-size: 1rem;
  }

  .features {
    display: flex;
    gap: 2rem;
  }

  .feature {
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }

  .feature-icon {
    font-size: 1.5rem;
  }

  .feature-text {
    display: flex;
    flex-direction: column;
    font-size: 0.875rem;
  }

  .feature-text strong {
    color: var(--text-primary);
  }

  .feature-text span {
    color: var(--text-muted);
  }

  .hero-visual {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .glow-orb {
    position: absolute;
    border-radius: 50%;
    filter: blur(60px);
    opacity: 0.5;
  }

  .orb-1 {
    width: 300px;
    height: 300px;
    background: var(--color-primary);
    top: -50px;
    right: -50px;
  }

  .orb-2 {
    width: 200px;
    height: 200px;
    background: var(--color-secondary);
    bottom: -30px;
    left: -30px;
  }

  .code-block {
    background: var(--bg-secondary);
    border: 1px solid var(--border-color);
    border-radius: var(--border-radius);
    padding: 2rem;
    position: relative;
    z-index: 1;
    font-family: "Fira Code", monospace;
    font-size: 0.9rem;
  }

  .code-block code {
    color: var(--text-primary);
  }

  .keyword {
    color: #c678dd;
  }
  .function {
    color: #61afef;
  }
  .string {
    color: #98c379;
  }

  @media (max-width: 900px) {
    .hero {
      grid-template-columns: 1fr;
      text-align: center;
    }

    .hero-content {
      align-items: center;
    }

    .hero-actions {
      justify-content: center;
    }

    .features {
      justify-content: center;
      flex-wrap: wrap;
    }

    .hero-visual {
      order: -1;
    }
  }
</style>
