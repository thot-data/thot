/** @type {import('tailwindcss').Config} */
import { colors, fontFamily, } from "./styles/theme.js";

module.exports = {
	darkMode: "selector",
	content: {
		files: ["*.html", "./src/**/*.rs"],
	},
	theme: {
		extend: {
			colors,
			fontFamily,
			transitionProperty: {
				"width": "width",
				"size": "width, height",
				"absolute-position": "top, bottom, left, right"
			}
		},
	},
	plugins: [
		require('@tailwindcss/forms'),
		require('tailwind-scrollbar'),
	],
}
