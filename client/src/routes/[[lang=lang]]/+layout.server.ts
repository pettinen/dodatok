import type { LayoutServerLoad } from "./$types";

console.log("hello from +layout.server.ts");

//  gets {
//     cookies, fetch, getClientAddress, locals, params, platform,
//     request, route, setHeaders, url, isDatRequest, depends, parent }
// locals comes from hooks.server.ts
export const load: LayoutServerLoad = ({ locals }) => {
    console.log("+layout.server.ts load()", locals);
};
