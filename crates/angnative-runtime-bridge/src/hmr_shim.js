/*
 * hmr_shim.js — HMR runtime for the embedded QuickJS realm.
 *
 * Mirrors `packages/contract/src/hmr.ts` without ES-module imports so it can be
 * evaluated by `runtime.load_script`. The shim is idempotent: a reload never
 * resets the module table, which is what makes same-realm state preservation
 * possible. No page/context teardown occurs on `replace()`.
 */
(function () {
  if (globalThis.__angnativeHmr) return;
  var modules = Object.create(null);
  globalThis.__angnativeHmr = {
    register: function (id, factory, hooks) {
      if (modules[id]) return false;
      var instance = factory();
      var h = hooks || {};
      modules[id] = {
        factory: factory,
        hooks: h,
        instance: instance,
        state: h.snapshot ? h.snapshot(instance) : undefined,
        generation: 1
      };
      return true;
    },
    replace: function (id, factory, hooks) {
      var current = modules[id];
      if (!current) return false;
      var state = current.hooks.snapshot
        ? current.hooks.snapshot(current.instance)
        : undefined;
      if (current.hooks.dispose) current.hooks.dispose(current.instance);
      var instance = factory();
      if (current.hooks.restore) current.hooks.restore(instance, state);
      current.factory = factory;
      current.hooks = hooks || {};
      current.instance = instance;
      current.state = state;
      current.generation += 1;
      return true;
    },
    instance: function (id) {
      return modules[id] ? modules[id].instance : undefined;
    },
    state: function (id) {
      return modules[id] ? modules[id].state : undefined;
    },
    generation: function (id) {
      return modules[id] ? modules[id].generation : 0;
    }
  };
})();
