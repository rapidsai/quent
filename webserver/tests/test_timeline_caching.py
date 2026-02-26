"""
Tests for the timeline caching module.
"""
import pytest
from unittest.mock import AsyncMock, MagicMock, patch

from fastapi import HTTPException

from webserver.timeline_caching import (
    determine_zoom_level,
    get_zoom_level_chunks,
    find_overlapping_chunks,
    fetch_chunk_with_cache,
    combine_chunk_result_data,
    _fetch_timeline_with_chunks,
    get_timeline_bins,
    get_timeline_bins_for_resource_group,
)


# ── Shared fixtures ───────────────────────────────────────────────────────────

@pytest.fixture
def binned_chunk_factory():
    """Factory for creating Binned timeline chunk response dicts (SingleTimelineResponse shape)."""
    def _make(start: float, end: float, num_bins: int = 10, capacity_values: dict | None = None):
        bin_duration = (end - start) / num_bins if num_bins > 0 else 1.0
        return {
            'config': {
                'span': {'start': start, 'end': end},
                'bin_duration': bin_duration,
                'num_bins': num_bins,
            },
            'data': {
                'Binned': {
                    'capacities_values': capacity_values or {
                        'memory': [float(i) for i in range(num_bins)]
                    },
                    'long_fsms': [],
                }
            }
        }
    return _make


@pytest.fixture
def binned_by_state_chunk_factory():
    """Factory for creating BinnedByState timeline chunk response dicts (SingleTimelineResponse shape)."""
    def _make(start: float, end: float, num_bins: int = 10):
        bin_duration = (end - start) / num_bins if num_bins > 0 else 1.0
        return {
            'config': {
                'span': {'start': start, 'end': end},
                'bin_duration': bin_duration,
                'num_bins': num_bins,
            },
            'data': {
                'BinnedByState': {
                    'capacities_states_values': {
                        'cpu': {
                            'running': [float(i) for i in range(num_bins)],
                            'idle': [float(i) * 0.5 for i in range(num_bins)],
                        }
                    },
                    'long_fsms': [],
                }
            }
        }
    return _make


# ── determine_zoom_level ──────────────────────────────────────────────────────

@pytest.mark.unit
def test_zoom_level_zero_range_returns_one():
    """Equal start/end (zero view range) should return zoom level 1."""
    assert determine_zoom_level(50.0, 50.0) == 1


@pytest.mark.unit
def test_zoom_level_negative_range_returns_one():
    """Reversed range (start > end) should return zoom level 1."""
    assert determine_zoom_level(80.0, 20.0) == 1


@pytest.mark.unit
def test_zoom_level_full_range():
    """Full 100% range: int(200/100) = 2."""
    assert determine_zoom_level(0.0, 100.0) == 2


@pytest.mark.unit
def test_zoom_level_half_range():
    """50% range: int(200/50) = 4."""
    assert determine_zoom_level(25.0, 75.0) == 4


@pytest.mark.unit
def test_zoom_level_quarter_range():
    """25% range: int(200/25) = 8."""
    assert determine_zoom_level(0.0, 25.0) == 8


@pytest.mark.unit
def test_zoom_level_20pct_range():
    """20% range: int(200/20) = 10, exactly at the ceiling."""
    assert determine_zoom_level(0.0, 20.0) == 10


@pytest.mark.unit
def test_zoom_level_small_range_clamped_to_ten():
    """Range smaller than 20% produces a value > 10, clamped to 10."""
    assert determine_zoom_level(0.0, 5.0) == 10


@pytest.mark.unit
def test_zoom_level_tiny_range_clamped_to_ten():
    """Very small range clamps to max zoom level 10."""
    assert determine_zoom_level(50.0, 50.1) == 10


@pytest.mark.unit
def test_zoom_level_not_zero_when_valid_range():
    """Any positive range should return at least 1."""
    for end in [10.0, 25.0, 50.0, 100.0]:
        assert determine_zoom_level(0.0, end) >= 1


# ── get_zoom_level_chunks ─────────────────────────────────────────────────────

@pytest.mark.unit
def test_zoom_chunks_level_one():
    """Zoom level 1 produces a single full-range chunk."""
    chunks = get_zoom_level_chunks(1)
    assert len(chunks) == 1
    assert chunks[0] == {
        'start': 0.0, 'end': 100.0,
        'zoom_level': 1, 'chunk_index': 0, 'num_bins': 200
    }


@pytest.mark.unit
def test_zoom_chunks_level_two():
    """Zoom level 2 produces two equal halves."""
    chunks = get_zoom_level_chunks(2)
    assert len(chunks) == 2
    assert chunks[0] == {'start': 0.0,  'end': 50.0,  'zoom_level': 2, 'chunk_index': 0, 'num_bins': 200}
    assert chunks[1] == {'start': 50.0, 'end': 100.0, 'zoom_level': 2, 'chunk_index': 1, 'num_bins': 200}


@pytest.mark.unit
def test_zoom_chunks_level_four_starts():
    """Zoom level 4 produces four quarter-sized chunks at the right offsets."""
    chunks = get_zoom_level_chunks(4)
    assert len(chunks) == 4
    assert chunks[0]['start'] == pytest.approx(0.0)
    assert chunks[1]['start'] == pytest.approx(25.0)
    assert chunks[2]['start'] == pytest.approx(50.0)
    assert chunks[3]['start'] == pytest.approx(75.0)


@pytest.mark.unit
@pytest.mark.parametrize("level", [1, 2, 3, 4, 5, 7, 10])
def test_zoom_chunks_last_chunk_ends_at_100(level):
    """Last chunk must always end exactly at 100.0 regardless of zoom level."""
    chunks = get_zoom_level_chunks(level)
    assert chunks[-1]['end'] == 100.0


@pytest.mark.unit
def test_zoom_chunks_default_num_bins_is_200():
    chunks = get_zoom_level_chunks(3)
    for chunk in chunks:
        assert chunk['num_bins'] == 200


@pytest.mark.unit
def test_zoom_chunks_custom_num_bins():
    chunks = get_zoom_level_chunks(2, num_bins=400)
    for chunk in chunks:
        assert chunk['num_bins'] == 400


@pytest.mark.unit
def test_zoom_chunks_indices_are_sequential():
    chunks = get_zoom_level_chunks(5)
    assert [c['chunk_index'] for c in chunks] == list(range(5))


@pytest.mark.unit
def test_zoom_chunks_zoom_level_field_matches():
    """Every chunk in the result must carry the correct zoom_level value."""
    for level in [1, 2, 4, 10]:
        chunks = get_zoom_level_chunks(level)
        for chunk in chunks:
            assert chunk['zoom_level'] == level


# ── find_overlapping_chunks ───────────────────────────────────────────────────

@pytest.mark.unit
def test_overlapping_empty_input():
    assert find_overlapping_chunks([], 0.0, 100.0) == []


@pytest.mark.unit
def test_overlapping_full_range_returns_all():
    chunks = get_zoom_level_chunks(4)
    assert find_overlapping_chunks(chunks, 0.0, 100.0) == chunks


@pytest.mark.unit
def test_overlapping_first_half_only():
    """Query [0, 50] overlaps only the first chunk of zoom-2; boundary is exclusive."""
    chunks = get_zoom_level_chunks(2)  # [0-50], [50-100]
    result = find_overlapping_chunks(chunks, 0.0, 50.0)
    # chunk[1].start=50 < end_pct=50 → False → not included
    assert len(result) == 1
    assert result[0]['chunk_index'] == 0


@pytest.mark.unit
def test_overlapping_second_half_only():
    chunks = get_zoom_level_chunks(2)
    result = find_overlapping_chunks(chunks, 50.0, 100.0)
    assert len(result) == 1
    assert result[0]['chunk_index'] == 1


@pytest.mark.unit
def test_overlapping_spanning_boundary():
    """A range straddling the midpoint must return both chunks."""
    chunks = get_zoom_level_chunks(2)
    result = find_overlapping_chunks(chunks, 25.0, 75.0)
    assert len(result) == 2


@pytest.mark.unit
def test_overlapping_no_overlap():
    """Chunk [50-100] does not overlap query [0, 50] (strict boundaries)."""
    chunks = [{'start': 50.0, 'end': 100.0, 'zoom_level': 2, 'chunk_index': 1, 'num_bins': 200}]
    assert find_overlapping_chunks(chunks, 0.0, 50.0) == []


@pytest.mark.unit
def test_overlapping_within_single_chunk():
    """Range contained entirely inside one chunk returns just that chunk."""
    chunks = get_zoom_level_chunks(4)  # [0-25], [25-50], [50-75], [75-100]
    result = find_overlapping_chunks(chunks, 30.0, 45.0)
    assert len(result) == 1
    assert result[0]['chunk_index'] == 1


@pytest.mark.unit
def test_overlapping_preserves_chunk_data():
    """Returned chunks are the same objects (not copies)."""
    chunks = get_zoom_level_chunks(2)
    result = find_overlapping_chunks(chunks, 0.0, 100.0)
    assert result == chunks


# ── fetch_chunk_with_cache ────────────────────────────────────────────────────

@pytest.mark.unit
async def test_fetch_cache_hit_returns_cached_value():
    """A cache hit should return the stored value without touching the backend."""
    cached_data = {'config': {}, 'data': {'Binned': {'capacities_values': {}}}}
    mock_cache = MagicMock()
    mock_cache.get.return_value = cached_data

    with patch('webserver.timeline_caching.timeline_cache', mock_cache):
        result = await fetch_chunk_with_cache('test-key', '/test/url', {'config': {}})

    assert result == cached_data
    mock_cache.get.assert_called_once_with('test-key')
    mock_cache.set.assert_not_called()


@pytest.mark.unit
async def test_fetch_cache_miss_calls_backend():
    """A cache miss should fetch from the backend via asyncio.to_thread."""
    fetched_data = {'config': {}, 'data': {'Binned': {'capacities_values': {}}}}
    mock_cache = MagicMock()
    mock_cache.get.return_value = None

    with (
        patch('webserver.timeline_caching.timeline_cache', mock_cache),
        patch('webserver.timeline_caching.asyncio.to_thread', new_callable=AsyncMock) as mock_thread,
    ):
        mock_thread.return_value = fetched_data
        result = await fetch_chunk_with_cache('test-key', '/test/url', {'config': {}})

    assert result == fetched_data


@pytest.mark.unit
async def test_fetch_cache_miss_stores_result():
    """After a cache miss the fetched value should be stored with the correct TTL."""
    fetched_data = {'config': {}, 'data': {'Binned': {'capacities_values': {}}}}
    mock_cache = MagicMock()
    mock_cache.get.return_value = None

    with (
        patch('webserver.timeline_caching.timeline_cache', mock_cache),
        patch('webserver.timeline_caching.asyncio.to_thread', new_callable=AsyncMock) as mock_thread,
    ):
        mock_thread.return_value = fetched_data
        await fetch_chunk_with_cache('test-key', '/test/url', {'config': {}})

    mock_cache.set.assert_called_once_with('test-key', fetched_data, ttl_seconds=3600)


@pytest.mark.unit
async def test_fetch_cache_miss_uses_rust_client():
    """asyncio.to_thread should invoke rust_client.post with the URL and body."""
    mock_cache = MagicMock()
    mock_cache.get.return_value = None
    mock_rust_client = MagicMock()
    mock_rust_client.post.return_value = {}

    with (
        patch('webserver.timeline_caching.timeline_cache', mock_cache),
        patch('webserver.timeline_caching.rust_client', mock_rust_client),
    ):
        await fetch_chunk_with_cache('key', '/analyzer/test', {'config': {'num_bins': 200}})

    mock_rust_client.post.assert_called_once_with('/analyzer/test', json={'config': {'num_bins': 200}})


@pytest.mark.unit
async def test_fetch_custom_ttl_passed_to_cache():
    """A custom ttl_seconds should be forwarded to cache.set."""
    mock_cache = MagicMock()
    mock_cache.get.return_value = None

    with (
        patch('webserver.timeline_caching.timeline_cache', mock_cache),
        patch('webserver.timeline_caching.asyncio.to_thread', new_callable=AsyncMock) as mock_thread,
    ):
        mock_thread.return_value = {'x': 1}
        await fetch_chunk_with_cache('key', '/url', {'config': {}}, ttl_seconds=7200)

    mock_cache.set.assert_called_once_with('key', {'x': 1}, ttl_seconds=7200)


# ── combine_chunk_result_data ─────────────────────────────────────────────────

@pytest.mark.unit
def test_combine_empty_chunks_raises():
    with pytest.raises(ValueError, match="No chunks to combine"):
        combine_chunk_result_data([], 0.0, 100.0)


@pytest.mark.unit
def test_combine_single_binned_full_range(binned_chunk_factory):
    """Single chunk covering the full requested range returns all bins."""
    chunk = binned_chunk_factory(start=0.0, end=10.0, num_bins=10)
    result = combine_chunk_result_data([chunk], start=0.0, end=10.0)

    assert result['config']['span'] == {'start': 0.0, 'end': 10.0}
    assert result['config']['num_bins'] == 10
    assert result['data']['Binned']['capacities_values']['memory'] == [float(i) for i in range(10)]


@pytest.mark.unit
def test_combine_single_binned_partial_range(binned_chunk_factory):
    """Requesting a sub-range extracts only the relevant bins."""
    # 10 bins over 0-10 s → bin_duration = 1.0 s/bin
    chunk = binned_chunk_factory(start=0.0, end=10.0, num_bins=10)
    result = combine_chunk_result_data([chunk], start=2.0, end=8.0)

    assert result['config']['span'] == {'start': 2.0, 'end': 8.0}
    assert result['config']['num_bins'] == 6
    assert result['data']['Binned']['capacities_values']['memory'] == [2.0, 3.0, 4.0, 5.0, 6.0, 7.0]


@pytest.mark.unit
def test_combine_two_binned_chunks_full_range(binned_chunk_factory):
    """Two adjacent chunks covering the full range are concatenated correctly."""
    chunk0 = binned_chunk_factory(start=0.0, end=5.0, num_bins=5,
                                  capacity_values={'memory': [0.0, 1.0, 2.0, 3.0, 4.0]})
    chunk1 = binned_chunk_factory(start=5.0, end=10.0, num_bins=5,
                                  capacity_values={'memory': [5.0, 6.0, 7.0, 8.0, 9.0]})

    result = combine_chunk_result_data([chunk0, chunk1], start=0.0, end=10.0)

    assert result['config']['num_bins'] == 10
    assert result['data']['Binned']['capacities_values']['memory'] == [float(i) for i in range(10)]


@pytest.mark.unit
def test_combine_chunks_sorted_by_start_time(binned_chunk_factory):
    """Chunks provided in reverse order should still be combined in the correct order."""
    chunk1 = binned_chunk_factory(start=5.0, end=10.0, num_bins=5,
                                  capacity_values={'memory': [5.0, 6.0, 7.0, 8.0, 9.0]})
    chunk0 = binned_chunk_factory(start=0.0, end=5.0, num_bins=5,
                                  capacity_values={'memory': [0.0, 1.0, 2.0, 3.0, 4.0]})

    result = combine_chunk_result_data([chunk1, chunk0], start=0.0, end=10.0)
    assert result['data']['Binned']['capacities_values']['memory'] == [float(i) for i in range(10)]


@pytest.mark.unit
def test_combine_non_overlapping_chunk_excluded(binned_chunk_factory):
    """A chunk whose range does not overlap the request window contributes no bins."""
    chunk = binned_chunk_factory(start=0.0, end=5.0, num_bins=5)
    result = combine_chunk_result_data([chunk], start=5.0, end=10.0)

    assert result['config']['num_bins'] == 0
    assert result['data']['Binned']['capacities_values'] == {}


@pytest.mark.unit
def test_combine_result_config_span(binned_chunk_factory):
    """Result config span reflects the requested start/end, not the chunk boundaries."""
    chunk = binned_chunk_factory(start=0.0, end=10.0, num_bins=10)
    result = combine_chunk_result_data([chunk], start=3.0, end=7.0)
    assert result['config']['span'] == {'start': 3.0, 'end': 7.0}


@pytest.mark.unit
def test_combine_result_config_bin_duration_from_first_chunk(binned_chunk_factory):
    """bin_duration in the result is taken from the first (sorted) chunk's config."""
    chunk = binned_chunk_factory(start=0.0, end=10.0, num_bins=10)
    result = combine_chunk_result_data([chunk], start=0.0, end=10.0)
    assert result['config']['bin_duration'] == pytest.approx(1.0)


@pytest.mark.unit
def test_combine_multiple_capacities(binned_chunk_factory):
    """All capacity columns are extracted and returned."""
    chunk = binned_chunk_factory(
        start=0.0, end=5.0, num_bins=5,
        capacity_values={
            'memory': [1.0, 2.0, 3.0, 4.0, 5.0],
            'cpu':    [10.0, 20.0, 30.0, 40.0, 50.0],
        }
    )
    result = combine_chunk_result_data([chunk], start=0.0, end=5.0)
    binned = result['data']['Binned']
    assert binned['capacities_values']['memory'] == [1.0, 2.0, 3.0, 4.0, 5.0]
    assert binned['capacities_values']['cpu'] == [10.0, 20.0, 30.0, 40.0, 50.0]


@pytest.mark.unit
def test_combine_single_binned_by_state_full_range(binned_by_state_chunk_factory):
    """BinnedByState chunk: all states are returned under capacities_states_values."""
    chunk = binned_by_state_chunk_factory(start=0.0, end=10.0, num_bins=10)
    result = combine_chunk_result_data([chunk], start=0.0, end=10.0)

    assert 'BinnedByState' in result['data']
    assert result['config']['num_bins'] == 10
    data = result['data']['BinnedByState']
    assert len(data['capacities_states_values']['cpu']['running']) == 10
    assert len(data['capacities_states_values']['cpu']['idle']) == 10


@pytest.mark.unit
def test_combine_binned_by_state_partial_range(binned_by_state_chunk_factory):
    """BinnedByState partial range extracts only matching bins per state."""
    # 10 bins over 0-10 s → bin_duration = 1.0
    chunk = binned_by_state_chunk_factory(start=0.0, end=10.0, num_bins=10)
    result = combine_chunk_result_data([chunk], start=0.0, end=5.0)

    assert result['config']['num_bins'] == 5
    data = result['data']['BinnedByState']
    assert len(data['capacities_states_values']['cpu']['running']) == 5


@pytest.mark.unit
def test_combine_two_binned_by_state_chunks(binned_by_state_chunk_factory):
    """Two BinnedByState chunks are merged per-capacity per-state."""
    chunk0 = binned_by_state_chunk_factory(start=0.0, end=5.0, num_bins=5)
    chunk1 = binned_by_state_chunk_factory(start=5.0, end=10.0, num_bins=5)

    result = combine_chunk_result_data([chunk0, chunk1], start=0.0, end=10.0)

    assert result['config']['num_bins'] == 10
    data = result['data']['BinnedByState']
    assert len(data['capacities_states_values']['cpu']['running']) == 10
    assert len(data['capacities_states_values']['cpu']['idle']) == 10


# ── _fetch_timeline_with_chunks ───────────────────────────────────────────────

# Shared mock return value for combine_chunk_result_data in these tests
_MOCK_COMBINED = {
    'config': {'span': {'start': 0.0, 'end': 100.0}, 'bin_duration': 0.5, 'num_bins': 200},
    'data': {
        'Binned': {
            'capacities_values': {},
            'long_fsms': [],
        }
    }
}
_MOCK_CHUNK = {
    'config': {'span': {'start': 0.0, 'end': 50.0}, 'bin_duration': 0.25, 'num_bins': 200},
    'data': {
        'Binned': {
            'capacities_values': {},
            'long_fsms': [],
        }
    }
}


@pytest.mark.unit
async def test_fetch_timeline_invalid_duration_zero():
    with pytest.raises(HTTPException) as exc_info:
        await _fetch_timeline_with_chunks(
            num_bins=200, start=0.0, end=10.0, duration=0.0,
            engine_id='eng1', query_id='q1', resource_id='res1',
            resource_type='resource',
        )
    assert exc_info.value.status_code == 400


@pytest.mark.unit
async def test_fetch_timeline_invalid_duration_negative():
    with pytest.raises(HTTPException) as exc_info:
        await _fetch_timeline_with_chunks(
            num_bins=200, start=0.0, end=10.0, duration=-1.0,
            engine_id='eng1', query_id='q1', resource_id='res1',
            resource_type='resource',
        )
    assert exc_info.value.status_code == 400


@pytest.mark.unit
async def test_fetch_timeline_end_equals_start():
    with pytest.raises(HTTPException) as exc_info:
        await _fetch_timeline_with_chunks(
            num_bins=200, start=5.0, end=5.0, duration=100.0,
            engine_id='eng1', query_id='q1', resource_id='res1',
            resource_type='resource',
        )
    assert exc_info.value.status_code == 400


@pytest.mark.unit
async def test_fetch_timeline_end_before_start():
    with pytest.raises(HTTPException) as exc_info:
        await _fetch_timeline_with_chunks(
            num_bins=200, start=10.0, end=5.0, duration=100.0,
            engine_id='eng1', query_id='q1', resource_id='res1',
            resource_type='resource',
        )
    assert exc_info.value.status_code == 400


@pytest.mark.unit
async def test_fetch_timeline_resource_cache_keys():
    """Cache keys for resource type must follow the expected format."""
    with (
        patch('webserver.timeline_caching.fetch_chunk_with_cache', new_callable=AsyncMock) as mock_fetch,
        patch('webserver.timeline_caching.combine_chunk_result_data', return_value=_MOCK_COMBINED),
    ):
        mock_fetch.return_value = _MOCK_CHUNK
        await _fetch_timeline_with_chunks(
            num_bins=200, start=0.0, end=100.0, duration=100.0,
            engine_id='eng1', query_id='q1', resource_id='res1',
            resource_type='resource',
            entity_type_name=None,
        )

    # 0-100% at full range → zoom level 2 → chunks [0-50], [50-100]
    calls = mock_fetch.call_args_list
    assert len(calls) == 2
    assert calls[0][0][0] == 'chunk:resource:eng1:q1:res1:z2:c0:0.000:50.000:200:None'
    assert calls[1][0][0] == 'chunk:resource:eng1:q1:res1:z2:c1:50.000:100.000:200:None'


@pytest.mark.unit
async def test_fetch_timeline_resource_cache_key_with_entity_type():
    """entity_type_name must appear in the resource cache key."""
    with (
        patch('webserver.timeline_caching.fetch_chunk_with_cache', new_callable=AsyncMock) as mock_fetch,
        patch('webserver.timeline_caching.combine_chunk_result_data', return_value=_MOCK_COMBINED),
    ):
        mock_fetch.return_value = _MOCK_CHUNK
        await _fetch_timeline_with_chunks(
            num_bins=200, start=0.0, end=100.0, duration=100.0,
            engine_id='eng1', query_id='q1', resource_id='res1',
            resource_type='resource',
            entity_type_name='QueryFsm',
        )

    cache_key_0 = mock_fetch.call_args_list[0][0][0]
    assert cache_key_0 == 'chunk:resource:eng1:q1:res1:z2:c0:0.000:50.000:200:QueryFsm'


@pytest.mark.unit
async def test_fetch_timeline_resource_group_cache_keys():
    """Cache keys for resource_group type must include resource_type_name."""
    with (
        patch('webserver.timeline_caching.fetch_chunk_with_cache', new_callable=AsyncMock) as mock_fetch,
        patch('webserver.timeline_caching.combine_chunk_result_data', return_value=_MOCK_COMBINED),
    ):
        mock_fetch.return_value = _MOCK_CHUNK
        await _fetch_timeline_with_chunks(
            num_bins=200, start=0.0, end=100.0, duration=100.0,
            engine_id='eng1', query_id='q1', resource_id='rg1',
            resource_type='resource_group',
            resource_type_name='thread',
            entity_type_name=None,
        )

    cache_key_0 = mock_fetch.call_args_list[0][0][0]
    assert cache_key_0 == 'chunk:resource_group:eng1:q1:rg1:z2:c0:0.000:50.000:200:thread:None'


@pytest.mark.unit
async def test_fetch_timeline_resource_group_cache_key_with_entity_type():
    with (
        patch('webserver.timeline_caching.fetch_chunk_with_cache', new_callable=AsyncMock) as mock_fetch,
        patch('webserver.timeline_caching.combine_chunk_result_data', return_value=_MOCK_COMBINED),
    ):
        mock_fetch.return_value = _MOCK_CHUNK
        await _fetch_timeline_with_chunks(
            num_bins=200, start=0.0, end=100.0, duration=100.0,
            engine_id='eng1', query_id='q1', resource_id='rg1',
            resource_type='resource_group',
            resource_type_name='thread',
            entity_type_name='QueryFsm',
        )

    cache_key_0 = mock_fetch.call_args_list[0][0][0]
    assert cache_key_0 == 'chunk:resource_group:eng1:q1:rg1:z2:c0:0.000:50.000:200:thread:QueryFsm'


@pytest.mark.unit
async def test_fetch_timeline_resource_fetch_url():
    """Fetch URL for resource type must target the single timeline endpoint."""
    with (
        patch('webserver.timeline_caching.fetch_chunk_with_cache', new_callable=AsyncMock) as mock_fetch,
        patch('webserver.timeline_caching.combine_chunk_result_data', return_value=_MOCK_COMBINED),
    ):
        mock_fetch.return_value = _MOCK_CHUNK
        await _fetch_timeline_with_chunks(
            num_bins=200, start=0.0, end=100.0, duration=100.0,
            engine_id='eng1', query_id='q1', resource_id='res1',
            resource_type='resource',
            entity_type_name=None,
        )

    fetch_url_0 = mock_fetch.call_args_list[0][0][1]
    assert fetch_url_0 == '/analyzer/engine/eng1/timeline/single'


@pytest.mark.unit
async def test_fetch_timeline_resource_group_fetch_url():
    """Fetch URL for resource_group uses the same single timeline endpoint."""
    with (
        patch('webserver.timeline_caching.fetch_chunk_with_cache', new_callable=AsyncMock) as mock_fetch,
        patch('webserver.timeline_caching.combine_chunk_result_data', return_value=_MOCK_COMBINED),
    ):
        mock_fetch.return_value = _MOCK_CHUNK
        await _fetch_timeline_with_chunks(
            num_bins=200, start=0.0, end=100.0, duration=100.0,
            engine_id='eng1', query_id='q1', resource_id='rg1',
            resource_type='resource_group',
            resource_type_name='thread',
            entity_type_name=None,
        )

    fetch_url_0 = mock_fetch.call_args_list[0][0][1]
    assert fetch_url_0 == '/analyzer/engine/eng1/timeline/single'


@pytest.mark.unit
async def test_fetch_timeline_entity_type_in_body():
    """When entity_type_name is provided it must appear in all fetch bodies."""
    with (
        patch('webserver.timeline_caching.fetch_chunk_with_cache', new_callable=AsyncMock) as mock_fetch,
        patch('webserver.timeline_caching.combine_chunk_result_data', return_value=_MOCK_COMBINED),
    ):
        mock_fetch.return_value = _MOCK_CHUNK
        await _fetch_timeline_with_chunks(
            num_bins=200, start=0.0, end=100.0, duration=100.0,
            engine_id='eng1', query_id='q1', resource_id='res1',
            resource_type='resource',
            entity_type_name='QueryFsm',
        )

    for call in mock_fetch.call_args_list:
        body = call[0][2]
        entry = body['entry']['Resource']
        assert entry['entity_filter']['entity_type_name'] == 'QueryFsm'


@pytest.mark.unit
async def test_fetch_timeline_no_entity_type_is_none_in_body():
    """When entity_type_name is None it must be None in the body's entity_filter."""
    with (
        patch('webserver.timeline_caching.fetch_chunk_with_cache', new_callable=AsyncMock) as mock_fetch,
        patch('webserver.timeline_caching.combine_chunk_result_data', return_value=_MOCK_COMBINED),
    ):
        mock_fetch.return_value = _MOCK_CHUNK
        await _fetch_timeline_with_chunks(
            num_bins=200, start=0.0, end=100.0, duration=100.0,
            engine_id='eng1', query_id='q1', resource_id='res1',
            resource_type='resource',
            entity_type_name=None,
        )

    for call in mock_fetch.call_args_list:
        body = call[0][2]
        entry = body['entry']['Resource']
        assert entry['entity_filter']['entity_type_name'] is None


@pytest.mark.unit
async def test_fetch_timeline_returns_combined_result():
    """The return value must be whatever combine_chunk_result_data produces."""
    expected = {
        'Binned': {
            'config': {'span': {'start': 0.0, 'end': 100.0}, 'bin_duration': 0.5, 'num_bins': 400},
            'capacities_values': {'memory': [1.0, 2.0]},
        }
    }
    with (
        patch('webserver.timeline_caching.fetch_chunk_with_cache', new_callable=AsyncMock) as mock_fetch,
        patch('webserver.timeline_caching.combine_chunk_result_data', return_value=expected),
    ):
        mock_fetch.return_value = _MOCK_CHUNK
        result = await _fetch_timeline_with_chunks(
            num_bins=200, start=0.0, end=100.0, duration=100.0,
            engine_id='eng1', query_id='q1', resource_id='res1',
            resource_type='resource',
        )

    assert result == expected


@pytest.mark.unit
async def test_fetch_timeline_correct_chunk_count_for_zoom():
    """Only the chunks that overlap the requested percentage range are fetched."""
    # duration=100, start=0, end=50 → start_pct=0, end_pct=50
    # zoom_level=4 (200/50=4) → chunks [0-25],[25-50],[50-75],[75-100]
    # overlapping [0,50): chunks 0 (0-25) and 1 (25-50) → 2 fetches
    with (
        patch('webserver.timeline_caching.fetch_chunk_with_cache', new_callable=AsyncMock) as mock_fetch,
        patch('webserver.timeline_caching.combine_chunk_result_data', return_value=_MOCK_COMBINED),
    ):
        mock_fetch.return_value = _MOCK_CHUNK
        await _fetch_timeline_with_chunks(
            num_bins=200, start=0.0, end=50.0, duration=100.0,
            engine_id='eng1', query_id='q1', resource_id='res1',
            resource_type='resource',
        )

    assert len(mock_fetch.call_args_list) == 2


# ── get_timeline_bins ─────────────────────────────────────────────────────────

@pytest.mark.unit
async def test_get_timeline_bins_delegates_correctly():
    """get_timeline_bins must call _fetch_timeline_with_chunks with resource_type='resource'."""
    expected = {'config': {}, 'data': {'Binned': {}}}
    with patch('webserver.timeline_caching._fetch_timeline_with_chunks', new_callable=AsyncMock) as mock_fetch:
        mock_fetch.return_value = expected
        result = await get_timeline_bins(
            num_bins=200, start=0.0, end=100.0, duration=100.0,
            engine_id='eng1', query_id='q1', resource_id='res1',
            entity_type_name='MyFsm',
        )

    mock_fetch.assert_called_once_with(
        num_bins=200, start=0.0, end=100.0, duration=100.0,
        engine_id='eng1', query_id='q1', resource_id='res1',
        resource_type='resource',
        entity_type_name='MyFsm',
    )
    assert result == expected


@pytest.mark.unit
async def test_get_timeline_bins_passes_none_entity_type():
    with patch('webserver.timeline_caching._fetch_timeline_with_chunks', new_callable=AsyncMock) as mock_fetch:
        mock_fetch.return_value = {}
        await get_timeline_bins(
            num_bins=200, start=0.0, end=10.0, duration=100.0,
            engine_id='eng1', query_id='q1', resource_id='res1',
            entity_type_name=None,
        )

    _, kwargs = mock_fetch.call_args
    assert kwargs['entity_type_name'] is None
    assert kwargs['resource_type'] == 'resource'


# ── get_timeline_bins_for_resource_group ──────────────────────────────────────

@pytest.mark.unit
async def test_get_timeline_bins_resource_group_delegates_correctly():
    """get_timeline_bins_for_resource_group passes resource_group_id as resource_id."""
    expected = {'config': {}, 'data': {'BinnedByState': {}}}
    with patch('webserver.timeline_caching._fetch_timeline_with_chunks', new_callable=AsyncMock) as mock_fetch:
        mock_fetch.return_value = expected
        result = await get_timeline_bins_for_resource_group(
            num_bins=200, start=0.0, end=100.0, duration=100.0,
            engine_id='eng1', query_id='q1', resource_group_id='rg1',
            resource_type_name='thread',
            entity_type_name='QueryFsm',
        )

    mock_fetch.assert_called_once_with(
        num_bins=200, start=0.0, end=100.0, duration=100.0,
        engine_id='eng1', query_id='q1', resource_id='rg1',
        resource_type='resource_group',
        entity_type_name='QueryFsm',
        resource_type_name='thread',
    )
    assert result == expected


@pytest.mark.unit
async def test_get_timeline_bins_resource_group_passes_resource_group_id():
    """resource_group_id is forwarded as resource_id to the inner function."""
    with patch('webserver.timeline_caching._fetch_timeline_with_chunks', new_callable=AsyncMock) as mock_fetch:
        mock_fetch.return_value = {}
        await get_timeline_bins_for_resource_group(
            num_bins=200, start=0.0, end=10.0, duration=100.0,
            engine_id='eng1', query_id='q1', resource_group_id='rg-xyz',
            resource_type_name='memory',
            entity_type_name=None,
        )

    _, kwargs = mock_fetch.call_args
    assert kwargs['resource_id'] == 'rg-xyz'
    assert kwargs['resource_type'] == 'resource_group'
    assert kwargs['resource_type_name'] == 'memory'
    assert kwargs['entity_type_name'] is None
