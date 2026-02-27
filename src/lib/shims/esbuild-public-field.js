/**
 * @param {object} obj
 * @param {string | symbol} key
 * @param {unknown} value
 */
export const __publicField = (obj, key, value) => {
  Object.defineProperty(obj, key, {
    value,
    enumerable: true,
    configurable: true,
    writable: true,
  });
  return value;
};
