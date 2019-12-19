SOURCE := https://equinox.bitwise.me/assets
ASSETS := aft_lounge_8k.raw \
          bethnal_green_entrance_4k.raw \
          between_bridges_1k.raw \
          between_bridges_2k.raw \
          between_bridges_4k.raw \
          between_bridges_8k.raw \
          blue_grotto_4k.raw \
          blue_lagoon_night_4k.raw \
          blue_lagoon_night_8k.raw \
          carpentry_shop_02_4k.raw \
          cayley_interior_8k.raw \
          cinema_lobby_4k.raw \
          cinema_lobby_8k.raw \
          green_point_park_8k.raw \
          lenong_2_1k.raw \
          lenong_2_2k.raw \
          lenong_2_4k.raw \
          moonless_golf_4k.raw \
          moonless_golf_8k.raw \
          noon_grass_2k.raw \
          noon_grass_4k.raw \
          noon_grass_8k.raw \
          old_outdoor_theater_4k.raw \
          paul_lobe_haus_4k.raw \
          sunny_vondelpark_2k.raw \
          sunny_vondelpark_4k.raw \
          sunny_vondelpark_8k.raw \
          aperture_filter_1.raw \
          aperture_filter_2.raw \
          aperture_filter_3.raw

download-assets: $(ASSETS)

$(ASSETS):
	curl -o $@ $(SOURCE)/$@
