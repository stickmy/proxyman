const {
  gray,
  blue,
  green,
  yellow,
  blackA,
  mauve,
  violet,
} = require("@radix-ui/colors");

/** @type {import('tailwindcss').Config} */
module.exports = {
  content: ["./index.html", "./src/**/*.{js,ts,jsx,tsx}"],
  theme: {
    extend: {
      colors: {
        ...gray,
        ...blue,
        ...green,
        ...yellow,
        ...blackA,
        ...mauve,
        ...violet,
      },
    },
  },
  plugins: [],
};
