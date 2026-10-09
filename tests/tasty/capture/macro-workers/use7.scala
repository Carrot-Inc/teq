object UseMacroWorkers7:
  def use(x: Any): Boolean = MacroWorkers.isString(x)
