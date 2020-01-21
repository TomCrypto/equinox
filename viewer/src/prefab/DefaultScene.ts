export default {
  name: "[Prefab] Default Scene",
  json: {
    metadata: {
      name: "[Prefab] Default Scene"
    },
    camera: {
      position: [6.8313575, 3.796789, -6.400666],
      direction: [-0.6979423, -0.3206336, 0.64036757],
      up_vector: [0, 1, 0],
      aperture: {
        type: "circle",
        radius: 0
      },
      focal_distance: 0.001,
      focal_curvature: 0,
      field_of_view: 0.2
    },
    raster: {
      width: 1280,
      height: 720,
      filter: {
        type: "blackman-harris"
      }
    },
    instance_list: {
      "glass-cylinder": {
        geometry: "cylinder",
        material: "glass",
        parameters: {
          x: 1,
          y: 0.955,
          z: 0
        },
        photon_receiver: false,
        sample_explicit: true,
        visible: true,
        medium: {
          extinction: [9e-8, 5e-8, 2e-9],
          refractive_index: 1.45
        },
        parent: null
      },
      "glass-sphere": {
        geometry: "sphere",
        material: "glass",
        parameters: {
          x: 0,
          y: 1.11,
          z: -1.5
        },
        photon_receiver: false,
        sample_explicit: true,
        visible: true,
        medium: {
          extinction: [1e-8, 6e-8, 6e-8],
          refractive_index: 1.65
        },
        parent: null
      },
      ground: {
        geometry: "ground",
        material: "lambertian",
        parameters: {},
        photon_receiver: true,
        sample_explicit: true,
        visible: true,
        medium: {
          extinction: [0, 0, 0],
          refractive_index: 1
        },
        parent: null
      },
      "matte-cube": {
        geometry: "cube",
        material: "matte",
        parameters: {
          x: -0.3,
          y: 0.81,
          z: 1.5
        },
        photon_receiver: true,
        sample_explicit: true,
        visible: true,
        medium: {
          extinction: [0, 0, 0],
          refractive_index: 1
        },
        parent: null
      }
    },
    geometry_list: {
      cube: {
        type: "translate",
        translation: ["x", "y", "z"],
        child: {
          type: "round",
          radius: 0.1,
          child: {
            type: "cuboid",
            dimensions: [0.699, 0.699, 0.699]
          }
        }
      },
      cylinder: {
        type: "translate",
        translation: ["x", "y", "z"],
        child: {
          type: "cylinder",
          height: 0.799,
          radius: 0.4
        }
      },
      ground: {
        type: "cuboid",
        dimensions: [3, 0.01, 3]
      },
      sphere: {
        type: "translate",
        translation: ["x", "y", "z"],
        child: {
          type: "sphere",
          radius: 0.799
        }
      }
    },
    material_list: {
      glass: {
        type: "dielectric",
        base_color: [1, 1, 1]
      },
      lambertian: {
        type: "lambertian",
        albedo: [0.5, 0.5, 0.5]
      },
      matte: {
        type: "oren-nayar",
        albedo: [0.25, 0.75, 0.25],
        roughness: 1
      }
    },
    environment_map: null,
    environment: {
      type: "solid",
      tint: [1, 1, 1]
    },
    display: {
      exposure: 0,
      saturation: 1,
      lens_flare_enabled: false,
      lens_flare_tiles_per_pass: 1
    },
    aperture: null,
    integrator: {
      hash_table_bits: 18,
      photons_per_pass: 100000,
      photon_rate: 0.5,
      max_search_radius: 0.05,
      min_search_radius: 0.005,
      alpha: 0.7,
      max_scatter_bounces: 3,
      max_gather_bounces: 5,
      geometry_precision: 0.001,
      geometry_pushback: 5
    }
  },
  thumbnail: "prefab-default-scene.jpg",
  timestamp: ""
};
