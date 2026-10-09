local function a() return 1 end
local function b() return 2 end
function c(x, y) return x, y end
check(a() == 1 and b() == 2, "several functions in one chunk")

local x, y = c(1)
check(x == 1 and y == nil, "missing parameters are nil")

local function stale(p)
    local q
    return q
end
stale(1)
c(7, 8)
check(stale() == nil, "locals start as nil even after earlier calls")

local function many() return 1, 2, 3 end
local t = { many() }
check(#t == 3 and t[3] == 3, "multiple results fill a constructor")
local function count(...) return 0 end
local function pass() return many() end
local p, q, r = pass()
check(p == 1 and q == 2 and r == 3, "tail calls return every result")

local function fact(n) if n <= 1 then return 1 end return n * fact(n - 1) end
check(fact(10) == 3628800, "recursion")

local function deep(n) if n == 0 then return 0 end return 1 + deep(n - 1) end
check(deep(300) == 300, "deep recursion grows the stack")

local obj = { n = 41 }
function obj:inc() self.n = self.n + 1 return self.n end
check(obj:inc() == 42, "method calls")
