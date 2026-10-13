# SevenPlus user interface strings, English (United States).
# Translations: copy this folder to e.g. de-DE/ and translate the values (ARC-008).

app-title = Calculator
app-name = SevenPlus Calculator

## Menus

menu-view = View
menu-edit = Edit
menu-help = Help
mode-standard = Standard
mode-scientific = Scientific
mode-programmer = Programmer
mode-statistics = Statistics
view-history = History
view-digit-grouping = Digit grouping
view-basic = Basic
view-unit-conversion = Unit conversion
view-date-calculation = Date calculation
view-worksheets = Worksheets
view-themes = Themes
themes-follow-system = Follow system (light/dark)
themes-reload = Reload themes
themes-open-folder = Open themes folder
view-reset-size = Reset size
view-zoom-in = Zoom in
view-zoom-out = Zoom out
view-window-frame = Window frame
frame-native = Native
frame-aero = Aero glass
frame-restart = Restart for rounded corners
edit-copy = Copy
edit-paste = Paste
edit-history = History
history-copy = Copy history
history-clear = Clear
edit-preferences = Preferences…
help-view = View Help
help-about = About Calculator
help-licenses = Licenses
menu-quit = Quit

## Worksheets

ws-mortgage = Mortgage
ws-vehicle-lease = Vehicle lease
ws-fuel-mpg = Fuel economy (mpg)
ws-fuel-l100km = Fuel economy (L/100 km)

## Mode panels

angle-degrees = Degrees
angle-radians = Radians
angle-grads = Grads
base-hex = Hex
base-dec = Dec
base-oct = Oct
base-bin = Bin
word-qword = Qword
word-dword = Dword
word-word = Word
word-byte = Byte
stat-count = Count = { $count }
stat-delete = Delete
display-memory = Memory is set
display-parens = Open parentheses: { $count }
display-label = Display
display-copy-hint = Double-click to copy
hint-hex = HEX
hint-dec = DEC
hint-oct = OCT
hint-bin = BIN
hint-qword = QWORD
hint-dword = DWORD
hint-word = WORD
hint-byte = BYTE
hint-deg = DEG
hint-rad = RAD
hint-grad = GRAD

## Errors (ENG-006)

error-divide-by-zero = Cannot divide by zero
error-undefined = Result is undefined
error-invalid-input = Invalid input
error-overflow = Overflow

## Extras panels

units-select = Select the type of unit you want to convert
units-from = From
units-to = To
date-select = Select the date calculation you want
date-mode-difference = Calculate the difference between two dates
date-mode-add = Add or subtract days to a specified date
date-from = From
date-to = To
date-difference-ymwd = Difference (years, months, weeks, days)
date-difference-days = Difference (days)
date-days = { $days ->
    [one] { $days } day
   *[other] { $days } days
}
date-add = Add
date-subtract = Subtract
date-years = Year(s)
date-months = Month(s)
date-days-input = Day(s)
date-result = Date
date-out-of-range = Date out of range
ws-select = Select the value you want to calculate
ws-calculate = Calculate

## About and preferences

about-version = Version { $version }
about-description = A calculator inspired by the Windows 7 Calculator, written in Rust.
about-font = Font: Inter, SIL Open Font License 1.1.
about-releases = Releases and updates
licenses-title = Third-party licenses
prefs-title = Preferences
prefs-theme = Theme
prefs-theme-light = Light theme
prefs-theme-dark = Dark theme
prefs-high-contrast = Use High Contrast when the system's high-contrast mode is on
prefs-numbers = Numbers
prefs-decimal = Decimal separator
prefs-group = Digit group separator
prefs-from-locale = From system ({ $sep })
prefs-copy-locale = Copy with the locale decimal separator
prefs-copy-grouping = Copy with digit grouping (when grouping is on)
prefs-open-folder = Open settings folder
theme-error = Theme error (click to dismiss):
settings-error = Settings were reset:

## Accessible key names (UI-020)

key-mem-clear = Memory clear
key-mem-recall = Memory recall
key-mem-store = Memory store
key-mem-add = Memory add
key-mem-sub = Memory subtract
key-back = Backspace
key-clear-entry = Clear entry
key-clear = Clear
key-negate = Positive negative
key-sqrt = Square root
key-percent = Percent
key-reciprocal = Reciprocal
key-divide = Divide by
key-multiply = Multiply by
key-minus = Minus
key-plus = Plus
key-equals = Equals
key-point = Decimal separator
key-inv = Inverse function
key-ln = Natural logarithm
key-exp-e = e to the power of x
key-open-paren = Left parenthesis
key-close-paren = Right parenthesis
key-int = Integer part
key-frac = Fractional part
key-sinh = Hyperbolic sine
key-asinh = Inverse hyperbolic sine
key-sin = Sine
key-asin = Arc sine
key-square = Square
key-factorial = Factorial
key-dms = Degrees minutes seconds
key-deg = Decimal degrees
key-cosh = Hyperbolic cosine
key-acosh = Inverse hyperbolic cosine
key-cos = Cosine
key-acos = Arc cosine
key-power = x to the power of y
key-root = y-th root of x
key-pi = Pi
key-two-pi = Two pi
key-tanh = Hyperbolic tangent
key-atanh = Inverse hyperbolic tangent
key-tan = Tangent
key-atan = Arc tangent
key-cube = Cube
key-cube-root = Cube root
key-fe = Scientific notation
key-exp = Exponent
key-mod = Modulo
key-log = Logarithm
key-pow10 = 10 to the power of x
key-rol = Rotate left
key-ror = Rotate right
key-or = Or
key-xor = Exclusive or
key-lsh = Left shift
key-rsh = Right shift
key-not = Not
key-and = And
key-mean = Average
key-mean-sq = Average of squares
key-sum = Sum
key-sum-sq = Sum of squares
key-sd-n = Population standard deviation
key-sd-n1 = Sample standard deviation
key-cad = Clear all data
key-add = Add to list
key-bit = Bit { $bit }
