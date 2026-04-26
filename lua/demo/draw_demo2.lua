

function process()
    draw_point(0.0, 0.0, {r=1.0, g=0.0, b=0.0}) -- Draw a point at the center of the field
    draw_text(0.0, 0.2, "Point") -- Draw text above the center point

    draw_point(0.8, 0.0, true, {r=1.0, g=1.0, b=0.0})
    draw_text(0.8, 0.2, "Point X")
end