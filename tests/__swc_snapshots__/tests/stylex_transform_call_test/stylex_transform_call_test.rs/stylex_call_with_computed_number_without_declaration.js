import _inject from "@stylexjs/stylex/lib/stylex-inject";
var _inject2 = _inject;
import { create } from '@stylexjs/stylex';
_inject2(".x1e2nbdu{color:red}", 3000);
_inject2(".x1t391ir{background-color:blue}", 3000);
const styles = {
    0: {
        color: "x1e2nbdu",
        $$css: true
    },
    1: {
        backgroundColor: "x1t391ir",
        $$css: true
    }
};
stylex(styles[0], styles[1]);