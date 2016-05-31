import {UPDATE_REQUEST, DATA, ERROR} from '../middleware/request'

export var fetch_indexes = {
    type: UPDATE_REQUEST,
    url: "/v1/log/index/latest",
    response_type: 'text',
    headers: {'Range': 'bytes=-65536'},
    decoder: x => x,
}

export function index(state={}, action) {
    switch(action.type) {
        case DATA:
            let lines = action.data.split('\n');
            // last item is always either empty or broken
            lines.pop();
            let rng = action.req.getResponseHeader("Content-Range")
            if(rng.substr(0, 8) != 'bytes 0-') {
                // if not a start of file, first item is broken too
                lines.shift();
            }
            let items = [];
            for(var line of lines) {
                let record
                try {
                    record = JSON.parse(line);
                } catch(e) {
                    console.log("Bad index line", line, e);
                    continue
                }
                items.push(record);
            }
            state = {...state, 'items': items};
            break;
    }
    return state;
}

export function log(state={}, action) {
    switch(action.type) {
        case DATA:
            state = {...state, 'text': action.data};
            break;
        case ERROR:
            state = {...state, 'text': action.error};
            break;
        case "show_mark":
            state = {...state, 'fetching': action.mark};
            break;
    }
    return state;
}

export function show_mark(mark) {
    return {
        type: "show_mark",
        mark: mark,
    }
}

export function view_from(mark) {
    let url;
    let off;
    if(mark.variant == 'Global') {
        url = "/v1/log/global/log." + mark.fields[0] + ".txt";
        off = mark.fields[1];
    } else if(mark.variant == 'Role') {
        url = "/v1/log/role/" + mark.fields[0] +
                "/log." + mark.fields[1] + ".txt";
        off = mark.fields[2];
    } else if(mark.variant == 'External') {
        url = "/v1/log/external/" + mark.fields[0];
        off = mark.fields[1];
    }
    return {
        type: UPDATE_REQUEST,
        url: url,
        response_type: 'text',
        headers: {'Range': 'bytes=' + off + '-' + (off + 65536)},
        decoder: x => x,
    }
}