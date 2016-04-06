JSON = require "JSON"
inspect = require "inspect"

function cycle(items)
    local i = 0
    local n = #items
    return function()
        i = i + 1
        if i > n then i = 1 end
        return items[i]
    end
end

function debugger()
    local x = {text=""}
    function x.object(self, title, data)
        self.text = self.text
            .. string.format('----- %s ----\n', title)
            .. inspect(data)
            .. "\n"
    end
    return x
end

function scheduler(state)
    local dbg = debugger()
    flag, value = pcall(_scheduler, state, dbg)
    if flag then
        return value, dbg.text
    else
        text = dbg.text .. string.format("\nError: %s", value)
        return nil, text
    end
end

function _scheduler(state, debugger)
    debugger:object("INPUT", state)
    local template_version = "v1"
    local runtime_version = "example-1" -- TODO(tailhook)
    local runtime = state.roles.pyapp.runtime[runtime_version]
    local req = runtime.required_processes
    local node_list = {"n1", "n2", "n3"}

    -- In this example we assume that all processes are equally costly
    -- TODO(tailhook) account already running things, so we do as little
    --                process migrations as possible
    local nodes = cycle(node_list)
    local counts = {}
    for name, number in pairs(req) do
        for i = 0,number,1 do
            local node_name = nodes()
            node = counts[node_name]
            if node == nil then
                node = {}
                counts[node_name] = node
            end
            oldval = node[name]
            if oldval == nil then oldval = 0 end
            node[name] = oldval + 1
        end
    end

    local nodes = {}
    for name, processes in pairs(counts) do
        items = {}
        for proc, num in pairs(processes) do
            proccfg = runtime.processes.daemons[proc]
            items[#items + 1] = {
                key=proc,
                image=proccfg.image,
                config=proccfg.config,
                instances=num,
            }
        end
        nodes[name] = {
            pyapp={
                daemons=items,
            },
        }
    end
    result = {
        role_metadata={
            pyapp={
                commands={},
                templates="v1",
                version=runtime_version,
            },
        },
        nodes=nodes,
    }
    return JSON:encode(result)
end
