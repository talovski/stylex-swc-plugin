import _inject from "@stylexjs/stylex/lib/stylex-inject";
var _inject2 = _inject;
import stylex from 'stylex';
_inject2(".xrkmrrc{background-color:red}", 3000);
_inject2(".x17fnjtu{width:var(--width,revert)}", 4000);
export const styles = {
    default: width => [{
        backgroundColor: "xrkmrrc",
        width: "x17fnjtu",
        $$css: true
    }, {
        "--width": (val => typeof val === "number" ? val + "px" : val != null ? val : "initial")(width)
    }]
};
