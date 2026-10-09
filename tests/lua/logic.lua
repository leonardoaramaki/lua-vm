local a = nil
local b = a or "default"
check(b == "default", "or picks the second value when the first is nil")
check((false or nil) == nil, "or with two falsy values")
check((1 and 2) == 2, "and returns the second value")
check((nil and 2) == nil, "and short-circuits")
check(not nil == true, "not nil")
check(not 0 == false, "0 is truthy")
check(UNDEFINED_GLOBAL == nil, "undefined globals are nil")
local v = DEBUG_FLAG or false
check(v == false, "or with an undefined global")
local x = 5
local clamp = x > 3 and 3 or x
check(clamp == 3, "and/or ternary")
