type RootDeferredGlobalsEnvironment = {
  requestAnimationFrame: (callback: FrameRequestCallback) => number;
  requestIdleCallback?: (callback: IdleRequestCallback) => number;
  setTimeout: (handler: TimerHandler, timeout?: number) => unknown;
};

export function scheduleRootDeferredGlobals(
  run: () => void,
  env: RootDeferredGlobalsEnvironment,
) {
  env.requestAnimationFrame(() => {
    if (env.requestIdleCallback) {
      env.requestIdleCallback(() => run());
      return;
    }

    env.setTimeout(run, 0);
  });
}
