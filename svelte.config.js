import adapter from '@sveltejs/adapter-static';

export default {
  kit: {
    adapter: adapter({
      pages: 'build',
      assets: 'build',
      fallback: undefined,
      precompress: false,
      strict: true
    }),
    // SPA mode: no server-side rendering
    prerender: {
      crawl: true,
      entries: ['/'],
      handleHttpError: 'warn'
    }
  }
};
