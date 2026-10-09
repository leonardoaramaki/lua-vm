local M = {}
local n = 0
function M.next()
    n = n + 1
    return n
end
return M
