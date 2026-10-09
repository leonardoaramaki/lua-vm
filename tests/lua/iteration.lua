local sum = 0
for i, v in ipairs({ 10, 20, 30, nil, 50 }) do sum = sum + i * v end
check(sum == 10 + 40 + 90, "ipairs stops at the first nil")

local keys = 0
local total = 0
for k, v in pairs({ 1, 2, x = 3, y = 4 }) do
    keys = keys + 1
    total = total + v
end
check(keys == 4 and total == 10, "pairs visits array and hash parts")

local order = {}
for k in pairs({ "a", "b", "c" }) do order[#order + 1] = k end
check(order[1] == 1 and order[2] == 2 and order[3] == 3, "pairs visits the array part in order")

local function range(n)
    local i = 0
    return function()
        i = i + 1
        if i <= n then return i end
    end
end
local seen = 0
for i in range(5) do seen = seen + i end
check(seen == 15, "generic for with a Lua iterator")

local nested = 0
for _, row in ipairs({ { 1, 2 }, { 3, 4 } }) do
    for _, v in ipairs(row) do nested = nested + v end
end
check(nested == 10, "nested generic for")

local found
for i, poi in pairs({ [7] = 2, [9] = 5 }) do
    if poi == 5 then found = i end
end
check(found == 9, "pairs over sparse keys")
check(next({}) == nil, "next on an empty table")
