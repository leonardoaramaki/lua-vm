local frame = 3725
local timer = string.format("%02d:%02d", math.floor(math.floor(frame / 60) / 60), math.floor(frame / 60) % 60)
check(timer == "01:02", "clock format from the game: " .. timer)
check(string.format("%0.2f kb", 2048 / 1024) == "2.00 kb", "memory format from the game")
check(string.format("%s=%d", "x", 7) == "x=7", "several arguments")
