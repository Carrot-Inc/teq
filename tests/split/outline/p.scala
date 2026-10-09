package p

import lib.Fields

def width(w: Int): String = Fields.field["width"](w, 2)
def height(h: Int): String = Fields.field["height"](h, 3)
