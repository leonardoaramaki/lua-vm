local t = {}
t[#t + 1] = "a"
t[#t + 1] = "b"
check(#t == 2 and t[2] == "b", "appending with #t + 1")

local big = {}
for i = 1, 120 do big[i] = i end
check(#big == 120, "length of a table filled by index")
local lit = { 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
    21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40,
    41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52, 53, 54, 55 }
check(#lit == 55 and lit[55] == 55 and lit[51] == 51, "constructors with more than 50 items")

local sparse = { [150] = 1, [265] = 11 }
check(sparse[150] == 1 and sparse[265] == 11 and #sparse == 0, "sparse integer keys")

local list = { 1, 2, 3 }
table.insert(list, 4)
table.insert(list, 1, 0)
check(#list == 5 and list[1] == 0 and list[5] == 4, "table.insert")
local removed = table.remove(list, 1)
check(removed == 0 and list[1] == 1 and #list == 4, "table.remove at a position")
check(table.remove(list) == 4 and #list == 3, "table.remove from the end")
check(table.remove({}) == nil, "table.remove on an empty table")

local m = { x = 1 }
m.x = nil
check(m.x == nil, "assigning nil removes the key")
local fkey = function() end
local byfn = { [fkey] = "f", [true] = "t" }
check(byfn[fkey] == "f" and byfn[true] == "t", "functions and booleans as keys")
