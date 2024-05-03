function transform(source, opts = {}) {
    return transformSync(source, {
        filename: opts.filename,
        parserOpts: {
            flow: 'all'
        },
        plugins: [
            stylexPlugin,
            opts
        ]
    });
}
describe('@stylexjs/babel-plugin', ()=>{
    describe('[metadata] plugin metadata', ()=>{
        test('stylex metadata is correctly set', ()=>{
            const output = transform(`
        import stylex from 'stylex';
        const styles = stylex.create({
          foo: {
            color: 'red',
            height: 5,
            ':hover': {
              start: 10,
            },
            '@media (min-width: 1000px)': {
              end: 5
            }
          },
        });

        const name = stylex.keyframes({
          from: {
            start: 0,
          },
          to: {
            start: 100,
          }
        });
      `);
            expect(output.metadata).toMatchInlineSnapshot(`
        {
          "stylex": [
            [
              "x1e2nbdu",
              {
                "ltr": ".x1e2nbdu{color:red}",
                "rtl": null,
              },
              3000,
            ],
            [
              "x1ycjhwn",
              {
                "ltr": ".x1ycjhwn{height:5px}",
                "rtl": null,
              },
              4000,
            ],
            [
              "xaiupp8",
              {
                "ltr": ".xaiupp8:hover{inset-inline-start:10px}",
                "rtl": null,
              },
              3130,
            ],
            [
              "x1uy60zq",
              {
                "ltr": "@media (min-width: 1000px){.x1uy60zq.x1uy60zq{inset-inline-end:5px}}",
                "rtl": null,
              },
              3200,
            ],
            [
              "x1g85oeb-B",
              {
                "ltr": "@keyframes x1g85oeb-B{from{inset-inline-start:0px;}to{inset-inline-start:100px;}}",
                "rtl": null,
              },
              1,
            ],
          ],
        }
      `);
        });
    });
});