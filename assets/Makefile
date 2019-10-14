SOURCE := https://equinox.bitwise.me/assets
ASSETS := blue_grotto_4k.raw \
		  cayley_interior_8k.raw \
		  green_point_park_8k.raw \
		  old_outdoor_theater_4k.raw \
		  paul_lobe_haus_4k.raw \
		  aperture.bin

download-assets: $(ASSETS)

$(ASSETS):
	curl -o $@ $(SOURCE)/$@
