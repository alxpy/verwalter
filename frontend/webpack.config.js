var webpack = require('webpack')
var BabiliPlugin = require('babili-webpack-plugin')
var DEV = process.env['NODE_ENV'] != 'production';
module.exports = {
    context: __dirname,
    entry: DEV ? [
        "./index",
        "webpack-dev-server/client?http://localhost:8080",
        "webpack/hot/only-dev-server",
    ] : "./index",
    output: {
        path: __dirname + "/../public/js",
        filename: "bundle.js",
        publicPath: '/common/js/',
    },
    module: {
        loaders: [{
            test: /\.khufu$/,
            loaders: ['babel-loader', 'khufu'],
            exclude: /node_modules/,
        }, {
            test: /\.js$/,
            loaders: ['babel-loader'],
            exclude: /node_modules/,
        }],
    },
    resolve: {
        modules: process.env.NODE_PATH.split(':').filter(x => x),
    },
    resolveLoader: {
        mainFields: ["webpackLoader", "main", "browser"],
        modules: process.env.NODE_PATH.split(':').filter(x => x),
    },
    devServer: {
        contentBase: '..',
        proxy: {
            '/v1/*': {
                target: 'http://localhost:8379',
                secure: false,
            },
            '/common/css/*': {
                target: 'http://localhost:8379',
                secure: false,
            },
            '/common/fonts/*': {
                target: 'http://localhost:8379',
                secure: false,
            },
        },
        publicPath: '/common/js/',
        hot: true,
        historyApiFallback: {
            index: 'public/index.html',
        },
    },
    plugins: [
        new webpack.LoaderOptionsPlugin({
            options: {
                khufu: {
                    static_attrs: !DEV,
                },
                babel: {
                    "plugins": [
                        "transform-strict-mode",
                        "transform-object-rest-spread",
                        "transform-es2015-block-scoping",
                        "transform-es2015-parameters",
                        "transform-es2015-destructuring",
                        "transform-es2015-arrow-functions",
                        "transform-es2015-block-scoped-functions",
                    ],
                },
            }
        }),
        new webpack.NoEmitOnErrorsPlugin(),
        new webpack.DefinePlugin({
            VERSION: JSON.stringify("v0.7.6"),
            "process.env.NODE_ENV": JSON.stringify(process.env['NODE_ENV']),
            DEBUG: DEV,
        }),
    ].concat(DEV ? [] : [
        new BabiliPlugin({}),
    ]),
}

