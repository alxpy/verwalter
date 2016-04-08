string = require "string"

function compare(a, b)
    if a:sub(1, 1) == 'v' then a = a:sub(2) end
    if b:sub(1, 1) == 'v' then b = b:sub(2) end
    local aiter = string.gmatch(a, "%w+")
    local biter = string.gmatch(b, "%w+")
    while true do
        local aitem = aiter()
        local bitem = biter()
        if aitem == nil then return bitem ~= null end
        if bitem == nil then return false end
        if aitem ~= bitem then
            if string.match(aitem, "^%d+$") then
                if string.match(bitem, "^%d+$") then
                    local anum = tonumber(aitem)
                    local bnum = tonumber(bitem)
                    return anum < bnum
                else -- numbers are always less than letters
                    return false
                end
            else
                if string.match("%d+", bitem) then
                    -- numbers are always less than letters
                    return true
                else
                    return aitem < bitem
                end
            end
        end
    end
end


----- SELF TESTS -----
assert(compare("v1", "v2"))
assert(not compare("v1.0", "v1"))
assert(not compare("v2", "v1"))
assert(compare("v2", "v10"))
assert(compare("v2", "v11"))
assert(not compare("v10", "v2"))
assert(not compare("v12", "v2"))
assert(compare("v1.1", "v2.1"))
assert(compare("v2.1", "v2.3"))
assert(not compare("v2.3", "v2.1"))
assert(compare("v2", "v2.1"))

-- false for equal versions
assert(not compare("v1", "v1"))
assert(not compare("v2", "v2"))
assert(not compare("v2.3.4", "v2.3.4"))

local _sorttable = {"v1.1.0", "v1.0", "v2.15", "v3.4.6", "v1", "v2.3",
                    "v1.1", "v10.0"}
table.sort(_sorttable, compare)
assert(_sorttable[1] == "v1")
assert(_sorttable[2] == "v1.0")
assert(_sorttable[3] == "v1.1")
assert(_sorttable[4] == "v1.1.0")
assert(_sorttable[5] == "v2.3")
assert(_sorttable[6] == "v2.15")
assert(_sorttable[7] == "v3.4.6")
assert(_sorttable[8] == "v10.0")
_sorttable = nil

return {
    compare=compare,
}