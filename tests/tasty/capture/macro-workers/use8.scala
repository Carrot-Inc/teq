object UseMacroWorkers8:
  def use(x: Any): Boolean = MacroWorkers.isString(x)
