SOURCE := https://equinox.bitwise.me/assets
ASSETS := blue_grotto_4k.raw \
		  cayley_interior_8k.raw \
		  green_point_park_8k.raw \
		  old_outdoor_theater_4k.raw \
		  paul_lobe_haus_4k.raw \
		  carpentry_shop_02_4k.raw \
		  bethnal_green_entrance_4k.raw \
		  blue_lagoon_night_4k.raw \
		  blue_lagoon_night_8k.raw \
		  cinema_lobby_4k.raw \
		  cinema_lobby_8k.raw \
		  moonless_golf_4k.raw \
		  moonless_golf_8k.raw \
		  aperture.raw \
		  aperture2.raw \
		  aperture3.raw

download-assets: $(ASSETS)

$(ASSETS):
	curl -o $@ $(SOURCE)/$@
