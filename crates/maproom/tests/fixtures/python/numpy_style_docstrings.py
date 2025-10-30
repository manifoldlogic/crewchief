"""Sample Python module with NumPy-style docstrings.

This module demonstrates various NumPy-style docstring patterns
for testing the docstring parser.
"""

import numpy as np


def compute_mean(data):
    """Compute the mean of an array.

    Parameters
    ----------
    data : array_like
        Input data array

    Returns
    -------
    float
        The mean value of the array
    """
    return np.mean(data)


def process_matrix(matrix, threshold, normalize=True):
    """Process a matrix with thresholding and optional normalization.

    Parameters
    ----------
    matrix : ndarray
        Input matrix to process
    threshold : float
        Threshold value for filtering
    normalize : bool, optional
        Whether to normalize the result (default is True)

    Returns
    -------
    ndarray
        Processed matrix

    Raises
    ------
    ValueError
        If matrix is empty
    TypeError
        If matrix is not a numpy array
    """
    if not isinstance(matrix, np.ndarray):
        raise TypeError("Matrix must be a numpy array")
    if matrix.size == 0:
        raise ValueError("Matrix cannot be empty")

    result = matrix.copy()
    result[result < threshold] = 0

    if normalize:
        result = result / np.max(result)

    return result


class DataProcessor:
    """Process and analyze numerical data.

    This class provides methods for statistical analysis and
    data transformation operations.

    Attributes
    ----------
    data : ndarray
        The data array being processed
    stats : dict
        Computed statistics for the data
    preprocessing_steps : list
        List of preprocessing operations applied
    """

    def __init__(self, data):
        """Initialize the data processor.

        Parameters
        ----------
        data : array_like
            Input data to process
        """
        self.data = np.asarray(data)
        self.stats = {}
        self.preprocessing_steps = []

    def transform(self, operation, **kwargs):
        """Apply a transformation to the data.

        Parameters
        ----------
        operation : str
            The type of transformation to apply
        **kwargs : dict
            Additional parameters for the transformation

        Returns
        -------
        ndarray
            Transformed data

        Raises
        ------
        ValueError
            If operation is not supported

        Notes
        -----
        Available operations include:
        - 'normalize': Scale data to [0, 1]
        - 'standardize': Scale to mean=0, std=1
        - 'log': Apply logarithmic transformation

        Examples
        --------
        >>> processor = DataProcessor([1, 2, 3, 4, 5])
        >>> result = processor.transform('normalize')
        >>> result
        array([0.  , 0.25, 0.5 , 0.75, 1.  ])
        """
        supported_ops = ['normalize', 'standardize', 'log']
        if operation not in supported_ops:
            raise ValueError(f"Unsupported operation: {operation}")

        if operation == 'normalize':
            result = (self.data - self.data.min()) / (self.data.max() - self.data.min())
        elif operation == 'standardize':
            result = (self.data - self.data.mean()) / self.data.std()
        else:
            result = np.log(self.data)

        self.preprocessing_steps.append(operation)
        return result


def fit_model(X, y, method='linear', regularization=0.01):
    """Fit a statistical model to data.

    Parameters
    ----------
    X : ndarray, shape (n_samples, n_features)
        Training data
    y : ndarray, shape (n_samples,)
        Target values
    method : {'linear', 'ridge', 'lasso'}, optional
        The fitting method to use (default is 'linear')
    regularization : float, optional
        Regularization strength (default is 0.01)

    Returns
    -------
    coefficients : ndarray, shape (n_features,)
        Fitted model coefficients
    intercept : float
        Model intercept term

    Raises
    ------
    ValueError
        If X and y have incompatible shapes

    See Also
    --------
    predict_model : Make predictions using fitted model
    evaluate_model : Evaluate model performance

    Notes
    -----
    The implementation uses ordinary least squares for linear method,
    and L2/L1 penalties for ridge/lasso respectively.

    References
    ----------
    .. [1] Hastie, T., Tibshirani, R., & Friedman, J. (2009).
           The Elements of Statistical Learning.
    """
    if X.shape[0] != y.shape[0]:
        raise ValueError("X and y must have same number of samples")

    # Simplified implementation
    coefficients = np.zeros(X.shape[1])
    intercept = 0.0

    return coefficients, intercept


def generate_samples(n_samples, distribution='normal', random_state=None):
    """Generate random samples from a distribution.

    Parameters
    ----------
    n_samples : int
        Number of samples to generate
    distribution : {'normal', 'uniform', 'exponential'}, optional
        Distribution type (default is 'normal')
    random_state : int or None, optional
        Random seed for reproducibility

    Yields
    ------
    float
        Next random sample

    Examples
    --------
    Generate 5 samples from normal distribution:

    >>> samples = list(generate_samples(5, random_state=42))
    >>> len(samples)
    5
    """
    if random_state is not None:
        np.random.seed(random_state)

    for _ in range(n_samples):
        if distribution == 'normal':
            yield np.random.randn()
        elif distribution == 'uniform':
            yield np.random.rand()
        else:
            yield np.random.exponential()
