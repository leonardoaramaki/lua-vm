require "mods.globals"
check(FROM_MODULE == "yes", "require runs the module")
local counter = require("mods.counter")
check(counter.next() == 1, "require returns the module table")
local again = require("mods.counter")
check(again == counter and again.next() == 2, "require caches modules")
package.preload["virtual"] = function(name) return { name = name } end
check(require("virtual").name == "virtual", "package.preload")
check(require("mods.globals") == true, "modules without a return value become true")
