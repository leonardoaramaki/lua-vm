local function counter()
    local n = 0
    return function()
        n = n + 1
        return n
    end
end
local c1, c2 = counter(), counter()
c1() c1()
check(c1() == 3, "upvalue keeps its value after the function returned")
check(c2() == 1, "each closure gets its own upvalue")

local function pair()
    local v = 0
    local function get() return v end
    local function set(x) v = x end
    return get, set
end
local get, set = pair()
set(10)
check(get() == 10, "closures share the same captured local")

local function outer()
    local v = "outer"
    return function()
        return function() return v end
    end
end
check(outer()()() == "outer", "nested closures reuse the parent's upvalue")

local fns = {}
for i = 1, 3 do
    local j = i * 10
    fns[i] = function() return j end
end
check(fns[1]() == 10 and fns[3]() == 30, "each loop iteration captures a fresh local")

local shared = 1
local function bump() shared = shared + 1 end
bump()
check(shared == 2, "writing an upvalue changes the open local")

function MakeThing()
    local thing = { hits = 0 }
    local function hit() thing.hits = thing.hits + 1 end
    thing.hit = hit
    return thing
end
local t = MakeThing()
t.hit() t.hit()
check(t.hits == 2, "factory pattern with local functions")
