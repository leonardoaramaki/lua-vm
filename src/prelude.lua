-- Parts of the standard library that are easier to write in Lua.
-- Runs once when the VM starts; __load_module is the native loader.
local load_module = __load_module
__load_module = nil

package = { loaded = {}, preload = {} }

function require(name)
    local module = package.loaded[name]
    if module ~= nil then
        return module
    end
    local loader = package.preload[name] or load_module(name)
    module = loader(name)
    if module == nil then
        module = true
    end
    package.loaded[name] = module
    return module
end
