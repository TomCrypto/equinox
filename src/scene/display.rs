use serde::{Deserialize, Serialize};
use smart_default::SmartDefault;

pub type CameraResponse = [[f32; 3]; 11];

#[derive(Debug, Deserialize, SmartDefault, Serialize)]
pub struct Display {
    #[default(0.0)]
    pub exposure: f32,
    #[default(1.0)]
    pub saturation: f32,
    #[default(None)]
    pub camera_response: Option<CameraResponse>,
}

#[allow(clippy::all)]
pub const AGFA_AGFACOLOR_HDC_100_PLUS: CameraResponse = [
    [
        0.007018532916957466,
        0.007719779383923759,
        0.007018532916957466,
    ],
    [5.699536243495123, 6.106492460029497, 5.699536243495123],
    [-51.557180961476874, -55.42999259464722, -51.557180961476874],
    [379.09484864584334, 395.59888188562365, 379.09484864584334],
    [-1765.5005057715, -1787.7800259028684, -1765.5005057715],
    [5228.584870711115, 5166.053730487841, 5228.584870711115],
    [-9993.351034368237, -9687.53893638046, -9993.351034368237],
    [12273.76274608924, 11725.262410547142, 12273.76274608924],
    [-9349.255114083908, -8831.204928526287, -9349.255114083908],
    [4017.989226046032, 3762.2452756156217, 4017.989226046032],
    [-744.4730677446942, -692.3192076668313, -744.4730677446942],
];

#[allow(clippy::all)]
pub const AGFA_ADVANTIX_100: CameraResponse = [
    [0.0117059874, 0.009161136924137961, 0.006131209509685431],
    [6.0964549901, 5.45759147128599, 4.520383043085695],
    [-60.2156935422, -51.52126606850824, -38.24377368444474],
    [470.0340439951, 401.3480905601841, 295.04031845527356],
    [-2278.524207539, -1943.8794959944828, -1428.5929955271552],
    [6932.1741887137, 5906.975337225334, 4333.929370430289],
    [-13502.0834887726, -11496.374044169179, -8402.73280092149],
    [16815.8817031052, 14315.865755677798, 10405.602309536953],
    [-12947.3392312413, -11026.731557806472, -7960.65210957379],
    [5611.9942652806, 4783.0584696473015, 3426.940554350958],
    [-1047.0286823081, -893.2068423752977, -634.815916929171],
];

#[allow(clippy::all)]
pub const AGFA_AGFACOLOR_FUTURA_100: CameraResponse = [
    [
        0.020035316087644545,
        0.02163104953273459,
        0.01491156729888263,
    ],
    [9.261633238267478, 9.403767585703035, 7.68087984300665],
    [-105.6082146171878, -106.71268001484174, -78.20782220223471],
    [842.9459614346545, 837.3404564991505, 587.8355941036615],
    [-4142.36008536895, -4039.9745233529047, -2771.3961271602684],
    [12752.102322582807, 12243.23616684905, 8279.369609215042],
    [
        -25079.168928620227,
        -23777.691213491664,
        -15928.054199815266,
    ],
    [31472.253794884324, 29543.871829897424, 19663.142696208222],
    [-24373.686017282154, -22699.8546233274, -15039.630856557953],
    [10611.686690911069, 9820.031210084391, 6484.970153502517],
    [-1986.4468100290444, -1828.6714577513474, -1204.723838335825],
];

#[allow(clippy::all)]
pub const AGFA_AGFACOLOR_FUTURA_II_100: CameraResponse = [
    [
        0.008123195824466344,
        0.007659608068962605,
        0.004121592043983018,
    ],
    [5.774376743240443, 5.782125361941275, 5.008937735649646],
    [-51.069973488008195, -51.460200492767925, -41.77833868294936],
    [387.1530895101831, 378.8797015240427, 288.89598137818103],
    [-1900.7111548807898, -1782.480194119776, -1218.9861314628051],
    [5930.981362325062, 5338.52109233578, 3181.3220593309843],
    [
        -11854.577236439949,
        -10306.525901483592,
        -5275.0080796430375,
    ],
    [15101.519498649592, 12765.754607536326, 5579.830529559026],
    [-11847.60444438599, -9792.004520371149, -3653.358207316165],
    [5214.7144704116035, 4232.414405773143, 1351.1600927172908],
    [-985.1870358177281, -787.8874739481588, -216.08905911059182],
];

#[allow(clippy::all)]
pub const AGFA_AGFACHROME_CT_PRECISA_100: CameraResponse = [
    [
        -0.002987119555079158,
        -0.0003604168597870961,
        -0.0019353246801603735,
    ],
    [0.9420614353666266, 0.310609568218869, 0.8206373272844625],
    [37.913978504911555, 14.391229485940869, 38.16790028388947],
    [-361.00369803194, -92.86978239378763, -354.1087377907583],
    [1881.6434390236277, 372.2296953438634, 1752.8102241518332],
    [-6157.798320559124, -950.8681410769533, -5398.1831091401355],
    [12870.559233369819, 1514.8703886401925, 10678.265270471127],
    [
        -17054.046792763857,
        -1449.1431199634815,
        -13532.474235993874,
    ],
    [13828.906493320728, 758.9502409690858, 10603.660072405124],
    [-6253.815525455406, -166.87141472274317, -4673.175608578899],
    [1207.7051146168385, 0.0, 885.2221417413882],
];

#[allow(clippy::all)]
pub const AGFA_AGFACHROME_RSX2_050: CameraResponse = [
    [
        -0.007066272947757231,
        -0.0010019607776970418,
        -0.005945600326253132,
    ],
    [3.0464677685359516, 0.8049168583057495, 2.8356803276190905],
    [53.29659457849657, 31.98011664136915, 47.41242384511315],
    [-611.293277981571, -234.9339831220821, -519.4024949902484],
    [3062.2921991898147, 861.1455093749497, 2502.1503427670746],
    [-8985.059614611768, -1885.0304246120959, -7117.550460668534],
    [16627.6308601509, 2554.864399487072, 12869.539993739016],
    [-19670.305311177734, -2102.2398284342357, -14968.5550685204],
    [14444.545807177492, 962.5913394180639, 10856.55292276665],
    [-6002.004716127791, -188.18123188605725, -4469.836491990014],
    [1078.860477403592, 0.0, 797.8613572851588],
];

#[allow(clippy::all)]
pub const CANON_OPTURA_981111: CameraResponse = [
    [
        -0.0006896197584271259,
        -0.0006896197584271259,
        -0.0006896197584271259,
    ],
    [2.6533390272371817, 2.6533390272371817, 2.6533390272371817],
    [28.740498451953993, 28.740498451953993, 28.740498451953993],
    [-459.9184558974929, -459.9184558974929, -459.9184558974929],
    [3011.7523575516648, 3011.7523575516648, 3011.7523575516648],
    [
        -11061.313343479844,
        -11061.313343479844,
        -11061.313343479844,
    ],
    [24667.025687843266, 24667.025687843266, 24667.025687843266],
    [
        -34062.894805286865,
        -34062.894805286865,
        -34062.894805286865,
    ],
    [28441.72918651906, 28441.72918651906, 28441.72918651906],
    [
        -13151.791841036083,
        -13151.791841036083,
        -13151.791841036083,
    ],
    [2585.022188203127, 2585.022188203127, 2585.022188203127],
];

#[allow(clippy::all)]
pub const KODAK_DSCS_3151: CameraResponse = [
    [
        -0.002484709017264911,
        -0.005697386034735911,
        -0.005769186895383326,
    ],
    [4.854269271124466, 5.060282387325152, 5.348900668827574],
    [-23.397348654663944, -20.83720150462696, -30.77633072686184],
    [117.30398692233358, 76.35955606284112, 164.13790809292973],
    [-384.62045714735945, -181.08647756471345, -537.107958777746],
    [733.3888494904811, 241.42672533683398, 1010.9556994375739],
    [-793.314124437394, -164.6527176024318, -1079.784162728939],
    [452.823450121252, 44.73825456785104, 609.7340378640562],
    [-106.03745069927857, 0.0, -141.50389392352136],
    [0.0, 0.0, 0.0],
    [0.0, 0.0, 0.0],
];

#[allow(clippy::all)]
pub const KODAK_EKTACHROME_64T: CameraResponse = [
    [
        0.006622672934392633,
        0.006266880993699112,
        0.00324766299225458,
    ],
    [
        -1.4512593286303739,
        -1.3036546202239885,
        -0.35138478779597077,
    ],
    [147.75426291920294, 108.91172387412439, 35.69248326182548],
    [-1362.0813354442134, -845.4813453667923, -39.40929004901869],
    [6282.115933833486, 3251.4396461664473, -901.3768710949679],
    [-17368.97466863335, -7359.883667351381, 5020.400306550692],
    [30484.826443375994, 10238.738820971535, -12739.30740936564],
    [-34272.56742220157, -8612.188616119607, 18385.81006537072],
    [23930.567140989595, 4020.609670431084, -15528.132231699674],
    [-9455.052106353329, -799.8502446650664, 7166.037147060753],
    [1615.8580731225866, 0.0, -1398.3654751354359],
];

#[allow(clippy::all)]
pub const KODAK_EKTACHROME_64: CameraResponse = [
    [
        0.002205127978427317,
        0.0008747165067352143,
        4.23291371689204e-05,
    ],
    [-0.14222551931624303, 0.0932306636791921, 0.1761439954991036],
    [20.12776304863503, 2.477074009960757, 0.7374342647566526],
    [74.71153677195676, 215.58765159183295, 98.45037183863056],
    [-1247.1490059932896, -1764.5630953627538, -699.1490538241696],
    [5449.334133431962, 6519.3862984533525, 2375.611934016658],
    [-12586.462215708605, -13867.599055162182, -4789.368653279848],
    [17242.0618999683, 18038.8780866379, 6002.764726066515],
    [-14083.92898835013, -14197.651128151443, -4594.606221511517],
    [6351.104074294953, 6219.749856147068, 1968.3817723198554],
    [-1218.6583466520678, -1165.3585813420723, -361.9967896498058],
];

#[allow(clippy::all)]
pub const KODAK_MAX_ZOOM_800: CameraResponse = [
    [
        0.009340504607283685,
        0.007844249520600493,
        0.004922302029995456,
    ],
    [5.650106827936737, 5.169643176310997, 4.078907942224075],
    [-53.174734567683856, -46.70805613938329, -30.79217380674213],
    [396.25677532529113, 348.7118577933471, 217.31160416323274],
    [-1858.1738205075528, -1647.7641495006449, -997.516842366618],
    [5531.265315401145, 4941.616978395748, 2938.4189055105767],
    [-10615.949132715641, -9541.078390986073, -5605.288057557071],
    [13084.69959960668, 11812.917584047069, 6879.805153084932],
    [-9997.58084892846, -9056.594196115308, -5239.494878019437],
    [4307.961856182775, 3913.0619023624145, 2251.8442622028315],
    [-799.9631986815324, -728.3397449075763, -417.37022642790237],
];

#[allow(clippy::all)]
pub const KODAK_PORTRA_100T: CameraResponse = [
    [
        0.008951044228247938,
        0.00813057632797427,
        0.005745405504250009,
    ],
    [4.775653369605477, 4.384170804063315, 3.5400922683706404],
    [-46.435822946170696, -40.329559965941044, -28.8782198014768],
    [363.25573150622546, 307.5534755643316, 218.27284369427332],
    [
        -1758.5993015173415,
        -1452.9133305625073,
        -1037.7675242018365,
    ],
    [5355.145809528534, 4322.552817307837, 3124.1466527105167],
    [-10445.645312883464, -8254.125575612845, -6048.323141984652],
    [13019.252327019787, 10094.296014905394, 7502.264600030656],
    [-10020.453100926818, -7640.106238710739, -5757.695520438289],
    [4336.940205925056, 3258.430223360831, 2488.5089410674796],
    [-807.2438339295642, -598.7487032159387, -463.0729347382043],
];

#[allow(clippy::all)]
pub const FUJIFILM_FCI: CameraResponse = [
    [
        0.0019708668454301564,
        0.004896849156720633,
        0.003750682988168301,
    ],
    [3.9082091354574575, 4.282679687078246, 3.429071456799252],
    [-29.48953870648596, -36.14071163651876, -27.429801154102602],
    [210.25300430501255, 270.80391647607337, 209.70376666940504],
    [-968.8189862249397, -1279.5439805673384, -1007.7910805623783],
    [2861.4987818582213, 3827.4867146078846, 3060.619100292047],
    [-5474.502193095845, -7360.799361008236, -5969.1981944471745],
    [6742.748520291843, 9066.946553882344, 7452.4242608466075],
    [-5155.924801914375, -6908.024790669296, -5753.878668087419],
    [2225.6220214236596, 2962.807199929415, 2501.14798657086],
    [-414.2954917760788, -546.8216107150558, -468.028814170636],
];

#[allow(clippy::all)]
pub const AGFA_AGFACOLOR_VISTA_100: CameraResponse = [
    [
        0.00817580813891173,
        0.0076763331566834835,
        0.004202330318265844,
    ],
    [5.781086184656941, 5.774803876851921, 4.976871061077094],
    [-51.19747379399449, -51.53258312726119, -41.164137481801504],
    [388.53689939278655, 380.1983038990546, 282.8769096548034],
    [
        -1910.2222804148303,
        -1792.2103036260114,
        -1188.2791724597064,
    ],
    [5970.44494108315, 5377.731629350111, 3092.837118812529],
    [-11953.819528850338, -10399.915746853272, -5123.933433714097],
    [15253.608795193923, 12900.651736976466, 5426.013997566801],
    [-11986.307494353445, -9908.101045782918, -3563.969239114886],
    [5283.842893033067, 4287.161552539891, 1325.2289612520951],
    [-999.6749575676357, -798.7647608295812, -213.59021373239992],
];